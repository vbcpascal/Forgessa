

#![allow(unused)]

use std::collections::{BTreeMap, BTreeSet};
use depile::analysis::control_flow::successor_blocks_impl;
use crate::ssa::{SSAKind, Block, Blocks};

pub type BlockSet = BTreeSet<usize>;
pub type DomTree = BTreeMap<usize, BlockSet>;
pub type ImmDomRel = BTreeMap<usize, Option<usize>>;


/// Returns nodes dominated by `block_idx`, i.e. `block_idx` dominates
/// `x` for `x` in return value.
pub fn dominate_nodes(domtree: &DomTree, block_idx: usize) -> BlockSet {
    let mut res: BlockSet = BlockSet::new();
    for (i, doms) in domtree {
        if doms.contains(&block_idx) { res.insert(*i); }
    }
    res
}

/// Returns `true` if `x` dom `y`.
pub fn dominate(domtree: &DomTree, x: usize, y: usize) -> bool {
    dominator(&domtree, y).contains(&x)
}

/// Returns dominators of `block_idx`, i.e. `x` dominates `block_idx`
/// for `x` in return value.
pub fn dominator(domtree: &DomTree, block_idx: usize) -> &BlockSet {
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

pub fn compute_domtree(blocks: &Blocks) -> DomTree {
    let mut domtree: DomTree = BTreeMap::new();
    domtree
}

pub fn compute_idom(domtree: &DomTree) -> ImmDomRel {
    let mut idoms = BTreeMap::new();
    for (i, _) in domtree { idoms.insert(*i, get_idom(*i, domtree)); }
    idoms
}

fn get_idom(block_idx: usize, domtree: &DomTree) -> Option<usize> {
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

#[macro_export]
macro_rules! map_b_bs {
    ($( $key: expr => $val: expr ),*) => {
         DomTree::from_iter([
             $( ($key, BTreeSet::from($val)), )*
         ])
    }
}

#[cfg(test)]
mod tests {
    use std::array::IntoIter;
    use std::collections::{BTreeMap, BTreeSet};
    use crate::analysis::domtree::{compute_domtree, compute_idom};
    use super::{BlockSet, DomTree};

    #[test]
    fn test_idom() {
        let domtree: DomTree = map_b_bs![
            0 => [0],
            1 => [0, 1],
            2 => [0, 1, 2],
            3 => [0, 1, 3],
            4 => [0, 1, 3, 4],
            5 => [0, 1, 3, 5],
            6 => [0, 1, 3, 6],
            7 => [0, 1, 7]
        ];
        let idoms = BTreeMap::from_iter([
            (0, None), (1, Some(0)), (2, Some(1)), (3, Some(1)),
            (4, Some(3)), (5, Some(3)), (6, Some(3)), (7, Some(1)),
        ]);
        assert_eq!(compute_idom(&domtree), idoms)
    }
}
