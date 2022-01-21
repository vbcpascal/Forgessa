use std::collections::{BTreeMap, BTreeSet};
use depile::ir::{Block, Function};
use depile::ir::instr::basic::Operand::Var;
use depile::ir::instr::Instr::Move;
use depile::ir::instr::InstrExt;
use crate::analysis::dom_frontier::{compute_dom_frontier, compute_dom_frontier_with_domtree};
use crate::analysis::domtree::{BlockMap, BlockSet, compute_domtree, compute_idom, ImmDomRel};
use crate::ssa::SSABlock;
use crate::ssa::SSAOpd::{Operand, Subscribed};

// pub fn place_phi(block: &SSABlock) -> SSABlock {
//
// }

/// Infer the place of phi function will be placed in `func`.
pub fn infer_phi(func: &Function) -> BTreeMap<usize, BTreeMap<String, PhiAtom>> {
    // Step 1: calculate dominance frontiers
    let dfs: BlockMap = compute_dom_frontier(func);

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
    let mut phi_instrs: BTreeMap<usize, BTreeMap<String, PhiAtom>> = BTreeMap::new();
    for i in 0..func.blocks.len() { phi_instrs.insert(i, BTreeMap::new()); }
    for (var, bs) in def_sites.iter() {
        let mut blocks: Vec<usize> = bs.clone();
        while !blocks.is_empty() {
            let b = blocks.pop().unwrap();

            for df in dfs.get(&b).unwrap() {
                let phis = phi_instrs.get_mut(df).unwrap();
                if !phis.contains_key(var) {
                    phis.insert(var.clone(), PhiAtom::new(var));
                    blocks.push(df.clone());
                }
                phis.get_mut(var).unwrap().insert(b);
            }
        }
    }

    phi_instrs
}

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
pub struct PhiAtom {
    pub var: String,
    /// Blocks generating this phi-node
    pub origins: BlockSet,
}

impl PhiAtom {
    pub fn new(var: &String) -> Self {
        PhiAtom { var: var.clone(), origins: BlockSet::new() }
    }
    pub fn insert(&mut self, block: usize) -> bool {
        self.origins.insert(block)
    }
}

pub struct PhiForge {
    pub domtree: BlockMap,
    pub imm_doms: ImmDomRel,
    pub dom_frontier: BlockMap,
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

        }
    }
}

#[cfg(test)]
mod test {
    use depile::ir::Function;
    use crate::analysis::phi::{find_defs, infer_phi};
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
        println!("{:?}", infer_phi(func));
    }


}