use depile::analysis::control_flow::HasBranchingBehaviour;
use depile::ir::Function;
use depile::ir::instr::InstrExt;
use crate::analysis::cfg::SimpleCfg;
use crate::analysis::domtree::BlockSet;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct NaturalLoop {
    /// The root of natural loop. This node *should* dominates all the
    /// nodes in `nodes`.
    pub root: usize,
    /// The blocks in natural loop.
    pub nodes: BlockSet,
    /// The back edge is from `back_edge` to `root`.
    pub back_edge: usize,
}

impl NaturalLoop {
    pub fn from(cfg: &SimpleCfg, from: usize, to: usize) -> NaturalLoop {
        fn visit(cfg: &SimpleCfg, node: usize, visited: &mut BlockSet) {
            if visited.contains(&node) { return; }
            visited.insert(node.clone());

            for prev in cfg.get_prevs(node) {
                visit(cfg, prev, visited);
            }
        }

        let mut visited = BlockSet::new();
        visited.insert(to);
        visit(cfg, from, &mut visited);
        NaturalLoop {root: to, nodes: visited, back_edge: from }
    }

    pub fn compute_loops<K: InstrExt>(func: &Function<K>) -> Vec<NaturalLoop>
        where K: InstrExt,
              K::Branching: HasBranchingBehaviour,
              K::Marker: HasBranchingBehaviour,
              K::Extra: HasBranchingBehaviour {
        let cfg = SimpleCfg::from(func.entry_block, func.blocks.as_slice());
        let mut loops: Vec<NaturalLoop> = Vec::new();
        for (from, tos) in &cfg.edges {
            for to in tos {
                if from > to { loops.push(NaturalLoop::from(&cfg, *from, *to)) }
            }
        }
        loops
    }

}

#[cfg(test)]
mod test {
    use crate::analysis::natural_loop::NaturalLoop;
    use crate::samples::{get_sample_functions, PRIME};

    #[test]
    fn test_loop() {
        let funcs = get_sample_functions(PRIME);
        let func = &funcs.functions[0];
        assert_eq!(NaturalLoop::compute_loops(func).len(), 2);
    }
}
