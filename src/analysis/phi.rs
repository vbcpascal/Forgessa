use std::cmp::max;
use std::collections::{BTreeMap, BTreeSet};
use depile::ir::{Block, Function, Instr};
use depile::ir::instr::basic::Operand;
use depile::ir::instr::basic::Operand::Var;
use depile::ir::instr::{BranchKind, InstrExt};
use depile::ir::instr::stripped::Functions;
use crate::to_isize;
use crate::analysis::cfg::SimpleCfg;
use crate::analysis::converter::block_convert;
use crate::analysis::panning::{Pannable, PannableBlock};
use crate::analysis::dom_frontier::compute_df_cfg;
use crate::analysis::domtree::{BlockMap, BlockSet, compute_domtree, compute_idom, ImmDomRel, root_of_domtree};
use crate::analysis::params::scan_parameters;
use crate::ssa::{Phi, SSABlock, SSAFunction, SSAFunctions, SSAInstr, SSAInterProc, SSAOpd};

/// Find all the variable definitions in `block`.
pub fn find_defs<K: InstrExt>(block: &Block<K>) -> BTreeSet<String>
    where K::Operand: HasVariableOperand {
    let mut vars = BTreeSet::new();
    for instr in block.instructions.iter() {
        match instr {
            Instr::Move { source: _, dest: dst }  => if dst.is_var() { vars.insert(dst.unwrap()); }
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
            SSAOpd::Operand(opd) => opd.get_var_name(),
            SSAOpd::Subscribed(var, _) => Some(var.clone()),
            _ => None,
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
    pub params: Vec<String>,
    pub cfg: SimpleCfg,
    pub domtree: BlockMap,
    pub imm_doms: ImmDomRel,
    pub dom_frontier: BlockMap,
    pub phi_cells: BlockPhiCells,
}

impl PhiForge {
    pub fn run(funcs: &Functions) -> (SSAFunctions, Vec<Vec<String>>) {
        fn count_instructions(func: &SSAFunction) -> usize {
            func.blocks.iter().fold(0, |x, block| x + block.instructions.len())
        }

        let mut curr_idx: usize = 0;
        let mut res = Vec::new();
        let mut params = Vec::new();

        for func in &funcs.functions {
            curr_idx = max(curr_idx, func.blocks[0].first_index);
            let (func_res, params_res) = PhiForge::run_func(&func, curr_idx);
            curr_idx += count_instructions(&func_res);
            res.push(func_res);
            params.push(params_res);
        }

        ( SSAFunctions { functions: res, entry_function: funcs.entry_function }, params )
    }

    fn run_func(func: &Function, instr_idx: usize) -> (SSAFunction, Vec<String>) {
        let mut forge = PhiForge::new(func);
        forge.infer_phi(func);
        forge.top_down_domtree();
        let mut func_phi = forge.place_phi_placeholder(func, instr_idx);
        forge.place_phi(&mut func_phi);
        forge.rename_phi(&mut func_phi);
        (func_phi, forge.params)
    }

    fn new(func: &Function) -> Self {
        let cfg = SimpleCfg::from(func.entry_block, func.blocks.as_slice());
        let domtree = compute_domtree(func);
        let imm_doms = compute_idom(&domtree);
        let dfs = compute_df_cfg(&domtree, &cfg);
        Self {
            params: scan_parameters(func),
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
        let phi_instrs = &mut self.phi_cells;
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
                .panning_forward_fill(self.phi_cells.get(&i).unwrap().len());
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
        for (i, b) in func.blocks.iter_mut().enumerate() {
            for j in 0..self.phi_cells.get(&i).unwrap().len() {
                *b.instructions.get_mut(j).unwrap() =
                    SSAInstr::Extra(Phi {
                        vars: Vec::new(),
                        blocks: Vec::new(),
                        dest: SSAOpd::NOpd
                    });
            }
        }
        func
    }

    pub fn rename_phi<'a>(&self, func: &'a mut SSAFunction) -> &'a mut SSAFunction {
        let mut rename_stack = RenameStack::new();
        let td_tree = self.top_down_domtree();
        let root = root_of_domtree(&self.domtree);
        for param in &self.params { rename_stack.request_push(param); }

        visit(self, root, func, &mut rename_stack, &td_tree);

        fn visit(forge: &PhiForge,
                 block_idx: usize,
                 func: &mut SSAFunction,
                 rename_stack: &mut RenameStack,
                 td_tree: &BlockMap) {
            let block: &mut SSABlock = func.blocks.get_mut(block_idx).unwrap();

            // Step 1: generate unique names and push them.
            for (j, (var, _)) in forge.phi_cells.get(&block_idx).unwrap().iter().enumerate() {
                let var_index: usize = rename_stack.request_push(var);
                match block.instructions.get_mut(j).unwrap() {
                    Instr::Extra(Phi {vars: _, blocks: _, dest}) =>
                        *dest = SSAOpd::Subscribed(var.clone(),  to_isize!(var_index)),
                    _ => panic!("Error"),
                }
            }

            // Step 2: rewrite names.
            for instr in block.instructions.iter_mut() {
                instr.rename_by(rename_stack);
            }

            // Step 3: fill in phi parameters of successor blocks.
            for succ in forge.cfg.get_succs(block_idx) {
                let succ_block = func.blocks.get_mut(succ).unwrap();
                for (j, (var, _)) in forge.phi_cells.get(&succ).unwrap().iter().enumerate() {
                    let instr = succ_block.instructions.get_mut(j).unwrap();
                    let var_idx = rename_stack.try_get(var);
                    push_phi_param(instr, var, var_idx, block_idx.try_into().unwrap());
                }
            }

            // Step 4: recurse on children.
            for child in td_tree.get(&block_idx).unwrap() {
                let mut rs = rename_stack.clone();
                visit(forge, *child, func, &mut rs, td_tree);
                for (var, cell) in &mut rename_stack.var_stacks {
                    cell.counter = rs.var_stacks.get(var).unwrap().counter;
                }
            }
        }
        func
    }
}

#[derive(Debug, Clone)]
pub struct RenameStack {
    var_stacks: BTreeMap<String, RenameStackCell>
}

impl RenameStack {
    fn new() -> Self { RenameStack { var_stacks: BTreeMap::new() } }

    fn try_get(&mut self, var: &String) -> isize {
        let cell = self.var_stack_mut(var);
        cell.stack.last().map_or(-1, |x| to_isize!(*x))
    }

    fn get(&mut self, var: &String) -> usize {
        let cell = self.var_stack_mut(var);
        *cell.stack.last().unwrap()
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

    fn var_stack_mut(&mut self, var: &String) -> &mut RenameStackCell {
        if !self.var_stacks.contains_key(var) {
            self.var_stacks.insert(var.clone(), RenameStackCell::new());
        }
        self.var_stacks.get_mut(var).unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct RenameStackCell {
    pub counter: usize,
    pub stack: Vec<usize>,
}

impl RenameStackCell {
    fn new() -> Self { RenameStackCell { counter: 0, stack: Vec::new() } }
}

/// Indicates definitions and operands can be renamed by [`RenameStack`].
pub trait Renameable {
    fn rename_by(&mut self, rename_stack: &mut RenameStack);
}

impl Renameable for SSAOpd {
    fn rename_by(&mut self, rename_stack: &mut RenameStack){
        match self {
            SSAOpd::Operand(opd) => {
                match opd {
                    Operand::Var(var, _) => {
                        let var_idx = rename_stack.get(var);
                        *self = SSAOpd::Subscribed(var.clone(), to_isize!(var_idx))
                    }
                    _ => { }
                }
            }
            _ => { }
        }
    }
}

impl Renameable for BranchKind<SSAOpd> {
    fn rename_by(&mut self, rename_stack: &mut RenameStack) {
        match self {
            BranchKind::If(opd) => opd.rename_by(rename_stack),
            BranchKind::Unless(opd) => opd.rename_by(rename_stack),
            _ => (),
        }
    }
}

impl Renameable for SSAInterProc {
    fn rename_by(&mut self, rename_stack: &mut RenameStack) {
        match self {
            SSAInterProc::PushParam(opd) => opd.rename_by(rename_stack),
            _ => (),
        }
    }
}

impl Renameable for SSAInstr {
    fn rename_by(&mut self, rename_stack: &mut RenameStack) {
        match self {
            Instr::Binary {op: _, lhs, rhs} =>
                { lhs.rename_by(rename_stack); rhs.rename_by(rename_stack); }
            Instr::Unary { op: _, operand } =>
                { operand.rename_by(rename_stack); }
            Instr::Branch(branching) =>
                { branching.method.rename_by(rename_stack); }
            Instr::Load(opd) =>
                { opd.rename_by(rename_stack); }
            Instr::Store {data, address} =>
                { data.rename_by(rename_stack); address.rename_by(rename_stack); }
            Instr::Write(opd) =>
                { opd.rename_by(rename_stack); }
            Instr::InterProc(interproc) =>
                { interproc.rename_by(rename_stack); }
            Instr::Move {source, dest} => {
                source.rename_by(rename_stack);
                match dest {
                    SSAOpd::Operand(Operand::Var(var, _)) =>
                        *dest = SSAOpd::Subscribed(var.clone(),
                                                   to_isize!(rename_stack.request_push(var))),
                    _ => ()
                }
            }
            _ => ()
        }
    }
}

fn push_phi_param(instr: &mut SSAInstr, var: &String, var_idx: isize, block_idx: isize) {
    match instr {
        Instr::Extra(Phi {vars, blocks, dest: _}) => {
            vars.push(SSAOpd::Subscribed(var.clone(), var_idx));
            blocks.push(block_idx.try_into().unwrap());
        }
        _ => panic!("Not phi instruction."),
    }
}

#[macro_export]
macro_rules! to_isize {
    ($num: expr) => { isize::try_from($num).unwrap() };
}

#[cfg(test)]
mod test {
    use std::io::{ Write, BufWriter };
    use depile::ir::Function;
    use crate::analysis::phi::{find_defs, PhiForge};
    use crate::samples::{ALL_SAMPLES, get_sample_functions, PRIME};

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

    #[test]
    fn test_phi_samples () {
        for (i, str) in ALL_SAMPLES.iter().enumerate() {
            let name = crate::samples::samples_str::ALL_SAMPLES[i].to_string().to_lowercase();
            let funcs = get_sample_functions(str);

            let file_path = format!("samples/stripped/{}.txt", name);
            let file = std::fs::File::create(file_path).unwrap();
            let mut writer = BufWriter::new(&file);
            write!(&mut writer, "{}", funcs).unwrap();

            let (res, _) = PhiForge::run(&funcs);
            let file_path = format!("samples/ssa/{}.txt", name);
            let file = std::fs::File::create(file_path).unwrap();
            let mut writer = BufWriter::new(&file);
            write!(&mut writer, "{}", res).unwrap();
        }
    }
}