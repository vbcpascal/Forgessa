#![allow(unused)]

use std::collections::{BTreeMap, BTreeSet};
use depile::analysis::control_flow::{ControlFlow, HasBranchingBehaviour, NextBlocks, successor_blocks_impl};
use depile::analysis::data_flow::{AnalysisRes, ForwardAnalysis};
use depile::analysis::lattice::JoinSemiLattice;
use depile::ir::{Block, Function};
use depile::ir::instr::InstrExt;
use crate::analysis::domtree::dominance_analysis::DomAnalysis;

/// A set of blocks
pub type BlockSet = BTreeSet<usize>;

/// Mapping block to a set of blocks, such as dominator as dominance frontier.
/// Can be build via macro [`map_b_bs!`].
pub type BlockMap = BTreeMap<usize, BlockSet>;
pub type ImmDomRel = BTreeMap<usize, Option<usize>>;

/// Returns nodes dominated by `block_idx`, i.e. `block_idx` dominates
/// `x` for `x` in return value.
pub fn dominate_nodes(domtree: &BlockMap, block_idx: usize) -> BlockSet {
    let mut res: BlockSet = BlockSet::new();
    for (i, doms) in domtree {
        if doms.contains(&block_idx) { res.insert(*i); }
    }
    res
}

/// Returns `true` if `x` dom `y`.
pub fn dominate(domtree: &BlockMap, x: usize, y: usize) -> bool {
    dominator(&domtree, y).contains(&x)
}

/// Returns dominators of `block_idx`, i.e. `x` dominates `block_idx`
/// for `x` in return value.
pub fn dominator(domtree: &BlockMap, block_idx: usize) -> &BlockSet {
    domtree.get(&block_idx).unwrap()
}

/// Returns nodes immediate dominated by `block_idx`, i.e. `block_idx`
/// immediate dominates `x` for `x` in return value.
pub fn imm_dominate_nodes(imm_doms: &ImmDomRel, block_idx: usize) -> BlockSet {
    let mut res: BlockSet = BlockSet::new();
    for (x, y) in imm_doms {
        // y.contains(block_idx)
        if y.map_or(false, |x| x == block_idx) { res.insert(*x); }
    }
    res
}

/// Returns immediate dominators of `block_idx`.
pub fn imm_dominators(imm_doms: &ImmDomRel, block_idx: usize) -> &Option<usize> {
    imm_doms.get(&block_idx).unwrap()
}

/// Returns the root of `domtree`.
pub fn root_of_domtree(domtree: &BlockMap) -> usize {
    for (i, doms) in domtree {
        if doms.len() == 1 { return *i; }
    }
    panic!("Root not found");
}

/// Compute dominator tree for function `func`.
pub fn compute_domtree<K: InstrExt>(func: &Function<K>) -> BlockMap
    where K::Branching: HasBranchingBehaviour,
          K::Marker: HasBranchingBehaviour,
          K::Extra: HasBranchingBehaviour {
    let res: Vec<AnalysisRes<DomAnalysis>> = DomAnalysis::run_forward(func);
    let mut domtree: BlockMap = BTreeMap::new();
    for (i, r) in res.iter().enumerate() {
        domtree.insert(i, r.out.get());
    }
    domtree
}

/// Compute immediate dominator for all blocks from `domtree`.
pub fn compute_idom(domtree: &BlockMap) -> ImmDomRel {
    let mut idoms = BTreeMap::new();
    for (i, _) in domtree { idoms.insert(*i, get_idom(*i, domtree)); }
    idoms
}

/// Compute immediate dominator for `block_idx`.
fn get_idom(block_idx: usize, domtree: &BlockMap) -> Option<usize> {
    let doms: &BlockSet = domtree.get(&block_idx).unwrap();
    for (i, ids) in domtree {
        let mut ids_ = ids.clone();
        ids_.insert(block_idx);
        if ids_.eq(doms) && !ids.contains(&block_idx) {
            return Some(*i);
        }
    }
    None
}

/// Macro to build a [`BlockMap`].
#[macro_export]
macro_rules! map_b_bs {
    ($( $key: expr => $val: expr ),*) => {
         BlockMap::from_iter([
             $( ($key, BTreeSet::from($val)), )*
         ])
    }
}

mod dominance_analysis {
    use std::borrow::Borrow;
    use depile::analysis::control_flow::{ControlFlow, ControlFlowExt};
    use depile::analysis::data_flow::ForwardAnalysis;
    use depile::analysis::lattice::JoinSemiLattice;
    use depile::ir::{Function, Block};
    use depile::ir::instr::InstrExt;
    use crate::analysis::domtree::BlockSet;

    #[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
    pub struct DomAnalysis { pub bs: BlockSet }

    impl DomAnalysis {
        pub fn empty() -> Self { Self{ bs: BlockSet::new() } }
        pub fn complete(size: usize) -> Self { Self{ bs: BlockSet::from_iter((0..size).into_iter()) } }
        pub fn get(&self) -> BlockSet { self.bs.clone() }
        pub fn insert(&mut self, x: usize) -> bool { self.bs.insert(x) }
    }

    impl<K: InstrExt> JoinSemiLattice<K> for DomAnalysis {
        fn bottom(env: &dyn ControlFlowExt<BlockKind=K>) -> Self {
            Self::complete(env.block_count())
        }

        fn join_assign(&mut self, other: Self) -> bool {
            let mut changed = false;
            for x in self.bs.clone() {
                if !other.bs.contains(&x) { changed |= self.bs.remove(&x); }
            }
            changed
        }
    }

    impl<K: InstrExt> ForwardAnalysis<K> for DomAnalysis {
        fn v_entry() -> Self { Self::empty() }

        fn transfer_function(block_idx: usize, block: &Block<K>, input: &Self, output: &mut Self) -> bool {
            let mut res = input.clone();
            res.insert(block_idx);
            <Self as JoinSemiLattice<K>>::join_assign(output, res)
            // output.join_assign::(res)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::array::IntoIter;
    use std::collections::{BTreeMap, BTreeSet};
    use std::process::id;
    use depile::ir::{Blocks, Function, Functions, Program};
    use depile::ir::instr::Kind;
    use crate::analysis::domtree::{compute_domtree, compute_idom};
    use super::{BlockSet, BlockMap};

    #[test]
    fn test_idom() {
        let domtree: BlockMap = map_b_bs![
            0 => [0], 1 => [0, 1], 2 => [0, 1, 2], 3 => [0, 1, 3],
            4 => [0, 1, 3, 4], 5 => [0, 1, 3, 5],
            6 => [0, 1, 3, 6], 7 => [0, 1, 7]
        ];
        let idoms = BTreeMap::from_iter([
            (0, None), (1, Some(0)), (2, Some(1)), (3, Some(1)),
            (4, Some(3)), (5, Some(3)), (6, Some(3)), (7, Some(1)),
        ]);
        assert_eq!(compute_idom(&domtree), idoms)
    }

    #[test]
    fn test_dom() {
        use depile::ir::program::read_program;
        use crate::samples;

        let program: Box<Program> = read_program(samples::PRIME).unwrap();
        let blocks: Blocks<depile::ir::instr::basic::Kind> = Blocks::try_from(program.as_ref()).unwrap();
        let funcs: Functions = blocks.functions().unwrap();
        let func = &funcs.functions[0];
        let domtree = compute_domtree(func);
        let idoms = compute_idom(&domtree);
        let idoms_ = BTreeMap::from_iter([
            (0, None), (1, Some(0)), (2, Some(1)), (3, Some(2)), (4, Some(3)),
            (5, Some(4)), (6, Some(4)) , (7, Some(6)) , (8, Some(4)),
            (9, Some(3)), (10, Some(9)), (11, Some(9)), (12, Some(1))
        ]);
        assert_eq!(idoms, idoms_);
    }
}
