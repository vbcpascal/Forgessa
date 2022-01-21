use depile::ir::Function;
use crate::analysis::dom_frontier::scfg::{SimpleCfg, to_simple_cfg};
use crate::analysis::domtree::{BlockSet, dominate, dominate_nodes, BlockMap, imm_dominators, ImmDomRel, compute_idom, root_of_domtree, compute_domtree};

pub fn compute_dom_frontier(func: &Function) -> BlockMap {
    let domtree = compute_domtree(func);
    let blocks = func.blocks.as_slice();
    let cfg = to_simple_cfg(func.entry_block, blocks);
    compute_df(&domtree, &cfg)
}

/// Compute dominance frontier (DF) for all nodes in `domtree`.
pub fn compute_df(domtree: &BlockMap, cfg: &SimpleCfg) -> BlockMap {
    let imm_doms: ImmDomRel = compute_idom(domtree);
    let root: usize = root_of_domtree(domtree);
    let mut res: BlockMap = BlockMap::new();
    df(root, domtree, &imm_doms, cfg, &mut res);
    res
}

/// Compute dominance frontier (DF) for `block_idx` and store the result in `dfs`.
fn df<'a>(block_idx: usize,
          domtree: &BlockMap,
          imm_doms: &ImmDomRel,
          cfg: &SimpleCfg,
          dfs: &'a mut BlockMap) -> &'a BlockSet {
    if dfs.contains_key(&block_idx) { return dfs.get(&block_idx).unwrap(); }
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
        for node in df(child, domtree, imm_doms, cfg, dfs) {
            if !dominate(domtree, block_idx, *node) { res.insert(*node); }
            if block_idx == *node { res.insert(*node); }
        }
    }
    dfs.insert(block_idx, res);
    return dfs.get(&block_idx).unwrap();
}

mod scfg {
    use std::collections::BTreeMap;
    use std::fmt::{Display, Formatter};
    use depile::analysis::control_flow::{HasBranchingBehaviour, successor_blocks_impl};
    use depile::ir::Block;
    use depile::ir::instr::InstrExt;
    use crate::analysis::domtree::BlockSet;

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

    pub fn to_simple_cfg<K>(entry: usize, blocks: &[Block<K>]) -> SimpleCfg
        where K: InstrExt,
              K::Branching: HasBranchingBehaviour,
              K::Marker: HasBranchingBehaviour,
              K::Extra: HasBranchingBehaviour {
        let mut edges = BTreeMap::new();
        for (i, _) in blocks.iter().enumerate() {
            let mut succs = BlockSet::new();
            for s in successor_blocks_impl(blocks, i) {
                // TODO: out of range
                if s >= blocks.len() { continue; }
                succs.insert(s);
            }
            edges.insert(i, succs);
        }
        SimpleCfg { entry, edges }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use super::{compute_df, compute_dom_frontier};
    use super::scfg::{SimpleCfg, to_simple_cfg};
    use crate::map_b_bs;
    use crate::samples::{get_sample_functions, PRIME, ALL_SAMPLES};
    use crate::analysis::domtree::BlockMap;

    #[test]
    fn test_df() {
        let domtree: BlockMap = map_b_bs![
            0 => [0], 1 => [0, 1], 2 => [0, 1, 2], 3 => [0, 1, 3],
            4 => [0, 1, 3, 4], 5 => [0, 1, 3, 5],
            6 => [0, 1, 3, 6], 7 => [0, 1, 7]
        ];
        let cfg: SimpleCfg = SimpleCfg {
            entry: 0,
            edges: map_b_bs![
                0 => [1], 1 => [2, 3], 2 => [7], 3 => [4, 5],
                4 => [6], 5 => [6]   , 6 => [7], 7 => [1]
            ]
        };
        let dfs = map_b_bs![
            0 => [] , 1 => [1], 2 => [7], 3 => [7],
            4 => [6], 5 => [6], 6 => [7], 7 => [1]
        ];
        assert_eq!(dfs, compute_df(&domtree, &cfg));
    }

    #[test]
    fn test_prime_cfg() {
        let funcs = get_sample_functions(PRIME);
        let func = &funcs.functions[0];
        let cfg = to_simple_cfg(func.entry_block, func.blocks.as_slice());
        let cfg_: SimpleCfg = SimpleCfg {
            entry: 0,
            edges: map_b_bs![
                0 => [1], 1 => [2, 12], 2 => [3], 3 => [4, 9],
                4 => [5, 6], 5 => [8], 6 => [7, 8], 7 => [8], 8 => [3],
                9 => [10, 11], 10 => [11], 11 => [1], 12 => []
            ]
        };
        assert_eq!(cfg, cfg_);
    }

    #[test]
    fn test_prime_df() {
        let funcs = get_sample_functions(PRIME);
        let func = &funcs.functions[0];
        let dfs = compute_dom_frontier(func);
        let dfs_ = map_b_bs![
            0 => [] , 1  => [1] , 2  => [1], 3  => [1, 3],
            4 => [3], 5  => [8] , 6  => [8], 7  => [8], 8 => [3],
            9 => [1], 10 => [11], 11 => [1], 12 => []
        ];
        assert_eq!(dfs, dfs_);
    }

    #[test]
    fn test_samples_df () {
        for s in ALL_SAMPLES {
            for func in get_sample_functions(s).functions.iter() {
                compute_dom_frontier(func);
            }
        }
    }
}
