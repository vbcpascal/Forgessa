use std::collections::BTreeMap;
use crate::analysis::dom_frontier::scfg::{SimpleCfg, to_simple_cfg};
use crate::analysis::domtree::{BlockSet, dominate, dominate_nodes, DomTree, imm_dominate_nodes, imm_dominators, ImmDomRel};
use crate::ssa::{Block, Function};

pub fn compute_dom_frontier(func: Function) -> BTreeMap<usize, BlockSet> {
    let mut res = BTreeMap::new();
    let blocks = func.blocks.as_slice();
    let cfg = to_simple_cfg(func.entry_block, blocks);

    res
}

fn df(block_idx: usize, domtree: &DomTree, imm_doms: &ImmDomRel, cfg: &SimpleCfg) -> BlockSet {
    let mut res: BlockSet = BlockSet::new();

    // compute Local(idx)
    for succ in cfg.edges.get(&block_idx).unwrap() {
        // !imm__.contains(block_idx)
        if !imm_dominators(imm_doms, *succ).map_or(false, |x| x == block_idx) {
            res.insert(*succ);
        }
    }
    // compute Up(idx)
    for child in dominate_nodes(domtree, block_idx) {
        if child == block_idx { continue; }
        for node in df(child, domtree, imm_doms, cfg) {
            if !dominate(domtree, block_idx, node) { res.insert(node); }
            if block_idx == node { res.insert(node); }
        }
    }
    res
}

mod scfg {
    use std::collections::BTreeMap;
    use std::fmt::{Display, Formatter};
    use depile::analysis::control_flow::successor_blocks_impl;
    use crate::analysis::domtree::BlockSet;
    use crate::ssa::Block;

    #[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
    pub struct SimpleCfg {
        pub entry: usize,
        pub edges: BTreeMap<usize, BlockSet>,
    }

    impl Display for SimpleCfg {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "CFG: entry {}\n", self.entry)?;
            for (x, ys) in &self.edges {
                for y in ys {
                    write!(f, "  {} -> {}\n", x, y)?;
                }
            }
            Ok(())
        }
    }

    pub fn to_simple_cfg(entry: usize, blocks: &[Block]) -> SimpleCfg {
        let mut edges = BTreeMap::new();
        for (i, _) in blocks.iter().enumerate() {
            let succs: BlockSet = successor_blocks_impl(blocks, i)
                .iter().cloned().collect();
            edges.insert(i, succs);
        }
        SimpleCfg { entry, edges }
    }
}

#[cfg(test)]
mod tests {
    use super::df;
    use super::scfg::SimpleCfg;
    use crate::analysis::domtree::{compute_idom, DomTree, ImmDomRel};
    use crate::{samples, domtree};
    use crate::ssa::Blocks;
    use std::collections::{BTreeMap, BTreeSet};
    use depile::ir::program::read_program;

    #[test]
    fn test_df() {
        let domtree: DomTree = domtree![
            0 => [0],
            1 => [0, 1],
            2 => [0, 1, 2],
            3 => [0, 1, 3],
            4 => [0, 1, 3, 4],
            5 => [0, 1, 3, 5],
            6 => [0, 1, 3, 6],
            7 => [0, 1, 7]
        ];
        println!("IMMDOMS ok");
        let imm_doms: ImmDomRel = compute_idom(&domtree);
        let cfg: SimpleCfg = SimpleCfg {
            entry: 0,
            edges: domtree![
                0 => [1],
                1 => [2, 3],
                2 => [7],
                3 => [4, 5],
                4 => [6],
                5 => [6],
                6 => [7],
                7 => [1]
            ]
        };
        let mut res = BTreeMap::new();
        for i in 0..=7 {
            res.insert(i, df(i, &domtree, &imm_doms, &cfg));
        }
        print!("{:?}", res);
    }
}