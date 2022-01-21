use std::collections::{BTreeMap, BTreeSet};
use depile::ir::{Block, Function};
use depile::ir::instr::basic::Operand::Var;
use depile::ir::instr::Instr::Move;
use depile::ir::instr::InstrExt;
use crate::analysis::dom_frontier::{compute_dom_frontier, compute_dom_frontier_with_domtree};
use crate::analysis::domtree::{BlockMap, BlockSet, compute_domtree, compute_idom, imm_dominators, ImmDomRel, root_of_domtree};
use crate::ssa::SSABlock;
use crate::ssa::SSAOpd::{Operand, Subscribed};

// pub fn place_phi(block: &SSABlock) -> SSABlock {
//
// }


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
    pub var: String,
    /// Blocks generating this phi-node
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

pub struct PhiForge {
    pub domtree: BlockMap,
    pub imm_doms: ImmDomRel,
    pub dom_frontier: BlockMap,
    pub rename_stack: BTreeMap<String, RenameStackCell>,
}

impl PhiForge {
    fn new(func: &Function) -> Self {
        let domtree = compute_domtree(func);
        let imm_doms = compute_idom(&domtree);
        let dfs = compute_dom_frontier_with_domtree(func, &domtree);
        Self {
            domtree: domtree,
            imm_doms: imm_doms,
            dom_frontier: dfs,
            rename_stack: BTreeMap::new(),
        }
    }

    /// Infer the place of phi function will be placed in `func`.
    pub fn infer_phi(&self, func: &Function) -> BTreeMap<usize, BTreeMap<String, PhiCell>> {
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
        let mut phi_instrs: BTreeMap<usize, BTreeMap<String, PhiCell>> = BTreeMap::new();
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
    pub fn traversal_order(&self) -> Vec<usize> {
        fn visit(block_idx: usize, imm_doms: &ImmDomRel, order: &mut Vec<usize>) {
            order.push(block_idx);
            for i in 0..imm_doms.len() {
                if imm_dominators(imm_doms, i).map_or(false, |x| x == block_idx) {
                    visit(i, imm_doms, order);
                }
            }
        }
        let mut order: Vec<usize> = Vec::new();
        let root: usize = root_of_domtree(&self.domtree);
        visit(root, &self.imm_doms, &mut order);
        order
    }

}

pub struct RenameStackCell {
    pub counter: usize,
    pub stack: Vec<usize>,
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
        let forge = PhiForge::new(func);
        println!("{:?}", forge.infer_phi(func));
        println!("{:?}", forge.traversal_order());
    }


}