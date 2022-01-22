use std::collections::{BTreeMap, BTreeSet};
use depile::analysis::control_flow::ControlFlowExt;
use depile::ir::{Block, Function, Instr};
use depile::ir::instr::basic::Operand::{Register, Var};
use depile::ir::instr::Instr::{Extra, Move};
use depile::ir::instr::InstrExt;
use crate::analysis::cfg::SimpleCfg;
use crate::analysis::converter::block_convert;
use crate::analysis::panning::{Pannable, PannableBlock};
use crate::analysis::dom_frontier::compute_df_cfg;
use crate::analysis::domtree::{BlockMap, BlockSet, compute_domtree, compute_idom, imm_dominators, ImmDomRel, root_of_domtree};
use crate::ssa::{Phi, SSABlock, SSAFunction, SSAInstr, SSAOpd};
use crate::ssa::SSAOpd::{Operand, Subscribed};

/// Find all the variable definitions in `block`.
pub fn find_defs<K: InstrExt>(block: &Block<K>) -> BTreeSet<String>
    where K::Operand: HasVariableOperand {
    let mut vars = BTreeSet::new();
    for instr in block.instructions.iter() {
        match instr {
            Move { source: _, dest: dst }  => if dst.is_var() { vars.insert(dst.unwrap()); }
            _ => { }
        }
    }
    vars
}

/// Indicates a variable can be got from this operand.
pub trait HasVariableOperand {
    fn get_var_name(&self) -> Option<String>;
    fn is_var(&self) -> bool { self.get_var_name().is_some() }
    fn unwrap(&self) -> String { self.get_var_name().unwrap() }
}

impl HasVariableOperand for depile::ir::instr::basic::Operand {
    fn get_var_name(&self) -> Option<String> {
        match self {
            Var(var, _) => Some(var.clone()),
            _ => None,
        }
    }
}

impl HasVariableOperand for crate::ssa::SSAOpd {
    fn get_var_name(&self) -> Option<String> {
        match self {
            Operand(opd) => opd.get_var_name(),
            Subscribed(var, _) => Some(var.clone()),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct PhiCell {
    /// The name of the phi node.
    pub var: String,
    /// Blocks generating this phi node.
    pub origins: BlockSet,
}

impl PhiCell {
    pub fn new(var: &String) -> Self {
        PhiCell { var: var.clone(), origins: BlockSet::new() }
    }
    pub fn insert(&mut self, block: usize) -> bool {
        self.origins.insert(block)
    }
}

pub type BlockPhiCells = BTreeMap<usize, BTreeMap<String, PhiCell>>;

pub struct PhiForge {
    pub cfg: SimpleCfg,
    pub domtree: BlockMap,
    pub imm_doms: ImmDomRel,
    pub dom_frontier: BlockMap,

    pub phi_cells: BlockPhiCells,
}

impl PhiForge {
    fn new(func: &Function) -> Self {
        let cfg = SimpleCfg::from(func.entry_block, func.blocks.as_slice());
        let domtree = compute_domtree(func);
        let imm_doms = compute_idom(&domtree);
        let dfs = compute_df_cfg(&domtree, &cfg);
        Self {
            cfg: cfg,
            domtree: domtree,
            imm_doms: imm_doms,
            dom_frontier: dfs,
            phi_cells: BTreeMap::new(),
        }
    }

    /// Infer the place of phi function will be placed in `func`.
    pub fn infer_phi(&mut self, func: &Function) -> &BlockPhiCells {
        // Step 1: calculate dominance frontiers
        let dfs: &BlockMap = &self.dom_frontier;

        // Step 2: find global names
        let mut defs: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
        for (i, block) in func.blocks.iter().enumerate() {
            defs.insert(i, find_defs(block));
        }

        let mut def_sites: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, _) in func.blocks.iter().enumerate() {
            for var in defs.get(&i).unwrap() {
                if !def_sites.contains_key(var) {
                    def_sites.insert(var.clone(), Vec::new());
                }
                def_sites.get_mut(var).unwrap().push(i);
            }
        }

        // Step 3: insert phi-functions
        let mut phi_instrs = &mut self.phi_cells;
        phi_instrs.clear();

        for i in 0..func.blocks.len() { phi_instrs.insert(i, BTreeMap::new()); }
        for (var, bs) in def_sites.iter() {
            let mut blocks: Vec<usize> = bs.clone();
            while !blocks.is_empty() {
                let b = blocks.pop().unwrap();

                for df in dfs.get(&b).unwrap() {
                    let phis = phi_instrs.get_mut(df).unwrap();
                    if !phis.contains_key(var) {
                        phis.insert(var.clone(), PhiCell::new(var));
                        blocks.push(df.clone());
                    }
                    phis.get_mut(var).unwrap().insert(b);
                }
            }
        }

        phi_instrs
    }

    /// Pre-order walk over dominator tree.
    pub fn top_down_domtree(&self) -> BlockMap {
        let mut res: BlockMap = BlockMap::new();
        for (i, _) in self.domtree.iter().enumerate() {
            res.insert(i, BlockSet::new());
        }
        for (i, j) in &self.imm_doms {
            if j.is_some() { res.get_mut(&j.unwrap()).unwrap().insert(*i); }
        }
        res
    }

    pub fn place_phi_placeholder(&self, func: &Function, instr_idx: usize) -> SSAFunction {
        let mut blocks: Vec<SSABlock> = Vec::new();
        let mut id = instr_idx;

        for (i, b) in func.blocks.iter().enumerate() {
            let offset = id - b.first_index;
            let block = block_convert(b)
                .pan(&|x| x + offset)
                .panning_forward_fill(self.phi_cells.get(&i).unwrap().len() * 2);
            id += block.instructions.len();
            blocks.push(block);
        }

        SSAFunction {
            parameter_count: func.parameter_count,
            local_var_count: 0, // TODO
            entry_block: func.entry_block,
            blocks: blocks,
        }
    }

    pub fn place_phi<'a>(&self, func: &'a mut SSAFunction) -> &'a mut SSAFunction {
        for (i, mut b) in func.blocks.iter_mut().enumerate() {
            for (j, (var, _)) in self.phi_cells.get(&i).unwrap().iter().enumerate() {
                *b.instructions.get_mut(2 * j).unwrap() =
                    SSAInstr::Extra(Phi { vars: Vec::new() });
            }
        }
        func
    }

    pub fn rename_phi<'a>(&self, func: &'a mut SSAFunction) -> &'a mut SSAFunction {
        // let order = self.traversal_order();
        // let mut rename_stack = RenameStack::new();
        //
        // for block_idx in order {
        //     let block: &mut SSABlock = func.blocks.get_mut(block_idx).unwrap();
        //
        //     // Step 1: generate unique names and push them.
        //     for (j, (var, _)) in self.phi_cells.get(&block_idx).unwrap().iter().enumerate() {
        //         let var_index: usize = rename_stack.request_push(var);
        //         let phi_instr_index: usize = block.first_index + var_index;
        //         *block.instructions.get_mut(2 * j + 1).unwrap() =
        //             SSAInstr::Move {
        //                 source: SSAOpd::Operand(Register(2 * j + phi_instr_index)),
        //                 dest: SSAOpd::Subscribed(var.clone(), var_index)
        //             }
        //     }
        //
        //
        // }
        func
    }
}

pub struct RenameStack {
    var_stacks: BTreeMap<String, RenameStackCell>
}

impl RenameStack {
    fn new() -> Self { RenameStack { var_stacks: BTreeMap::new() } }

    fn init(&mut self, vars: &Vec<String>) {
        for var in vars {
            self.var_stacks.insert(var.clone(), RenameStackCell::new());
        }
    }

    fn var_stack_mut(&mut self, var: &String) -> &mut RenameStackCell {
        if !self.var_stacks.contains_key(var) {
            self.var_stacks.insert(var.clone(), RenameStackCell::new());
        }
        self.var_stacks.get_mut(var).unwrap()
    }

    fn request_push(&mut self, var: &String) -> usize {
        let id = self.request(var);
        self.push_var(var, id);
        id
    }

    fn request(&mut self, var: &String) -> usize {
        let cell = self.var_stack_mut(var);
        let id = cell.counter;
        cell.counter += 1;
        id
    }

    fn push_var(&mut self, var: &String, var_idx: usize) {
        let cell = self.var_stack_mut(var);
        cell.stack.push(var_idx);
    }
}

pub struct RenameStackCell {
    pub counter: usize,
    pub stack: Vec<usize>,
}

impl RenameStackCell {
    fn new() -> Self { RenameStackCell { counter: 0, stack: Vec::new() } }
}

#[cfg(test)]
mod test {
    use depile::ir::Function;
    use crate::analysis::phi::{find_defs, PhiForge};
    use crate::samples::{get_sample_functions, PRIME};

    #[test]
    fn test_find_defs() {
        let funcs = get_sample_functions(PRIME);
        let func: &Function = &funcs.functions[0];
        for (i, block) in func.blocks.iter().enumerate() {
            println!("Block {}: {:?}", i, find_defs(block));
        }
    }

    #[test]
    fn test_phi_instrs() {
        let funcs = get_sample_functions(PRIME);
        let func: &Function = &funcs.functions[0];
        let mut forge = PhiForge::new(func);
        println!("{:?}", forge.infer_phi(func));
        println!("{:?}", forge.top_down_domtree());
        let mut func_phi = forge.place_phi_placeholder(func, func.blocks[0].first_index);
        forge.place_phi(&mut func_phi);
        forge.rename_phi(&mut func_phi);
        println!("{}", func_phi);
    }


}