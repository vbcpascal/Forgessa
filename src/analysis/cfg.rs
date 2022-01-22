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

impl SimpleCfg {
    pub fn from<K>(entry: usize, blocks: &[Block<K>]) -> Self
        where K: InstrExt,
              K::Branching: HasBranchingBehaviour,
              K::Marker: HasBranchingBehaviour,
              K::Extra: HasBranchingBehaviour {
        let mut edges = BTreeMap::new();
        for (i, _) in blocks.iter().enumerate() {
            let mut succs = BlockSet::new();
            for s in successor_blocks_impl(blocks, i) {
                succs.insert(s);
            }
            edges.insert(i, succs);
        }
        Self { entry, edges }
    }

    pub fn get_succs(&self, block_idx: usize) -> BlockSet {
        self.edges.get(&block_idx).unwrap().clone()
    }

    pub fn get_prevs(&self, block_idx: usize) -> BlockSet {
        let mut res = BlockSet::new();
        for (b, blocks) in self.edges.iter() {
            if blocks.contains(&block_idx) { res.insert(*b); }
        }
        res
    }
}

#[cfg(test)]
mod test {
    use std::collections::BTreeSet;
    use crate::map_b_bs;
    use crate::analysis::domtree::{BlockMap, BlockSet};
    use crate::analysis::cfg::SimpleCfg;
    use crate::samples::{get_sample_functions, PRIME};

    #[test]
    fn test_prime_cfg() {
        let funcs = get_sample_functions(PRIME);
        let func = &funcs.functions[0];
        let cfg = SimpleCfg::from(func.entry_block, func.blocks.as_slice());
        let cfg_ = SimpleCfg {
            entry: 0,
            edges: map_b_bs![
                0 => [1], 1 => [2, 12], 2 => [3], 3 => [4, 9],
                4 => [5, 6], 5 => [8], 6 => [7, 8], 7 => [8], 8 => [3],
                9 => [10, 11], 10 => [11], 11 => [1], 12 => []
            ]
        };
        assert_eq!(cfg, cfg_);
        assert_eq!(cfg.get_succs(6), BlockSet::from([7, 8]));
        assert_eq!(cfg.get_prevs(3), BlockSet::from([2, 8]));
    }
}