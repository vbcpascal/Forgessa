use depile::ir::Instr;
use depile::ir::instr::Branching;
use crate::analysis::panning::panning_function;
use crate::ssa::{Phi, SSABlock, SSAFunction, SSAInstr};

pub struct BlockInserter {
    pub insert_idx: usize,
}

impl BlockInserter {
    pub fn new(idx: usize) -> Self { BlockInserter { insert_idx: idx } }

    pub fn run(func: &mut SSAFunction, insert_idx: usize) {
        BlockInserter::new(insert_idx).modify_function(func);
        panning_function(func, func.blocks[0].first_index);
    }

    pub fn modify_function(&self, func:&mut SSAFunction) {
        let mut blocks = Vec::new();

        for (i, block) in func.blocks.iter_mut().enumerate() {
            if self.insert_idx == i {
                blocks.push(SSABlock { first_index: 0, instructions: Vec::new().into_boxed_slice() })
            }
            self.modify_block(block, i);
            blocks.push(block.clone());
        }

        func.blocks = blocks;
        if func.entry_block > self.insert_idx { func.entry_block += 1; }
    }

    pub fn modify_block(&self, block:&mut SSABlock, block_idx: usize) {
        for instr in block.instructions.iter_mut() {
            self.modify_instr(instr, block_idx);
        }
    }

    pub fn modify_instr(&self, instr: &mut SSAInstr, block_idx: usize) {
        match instr {
            Instr::Branch(Branching {method: _, dest}) =>
                *dest = helper::modify(block_idx, self.insert_idx, *dest),
            Instr::Extra(Phi {vars: _, blocks, dest: _}) =>
                for block in blocks {
                    *block = if *block > self.insert_idx { *block + 1 } else { *block };
                }
            _ => ()
        }
    }
}

mod helper {
    pub fn modify(block_idx: usize, insert_idx: usize, dest: usize) -> usize {
        if dest < insert_idx || dest == insert_idx && block_idx < insert_idx { dest }
        else { dest + 1 }
    }
}

#[cfg(test)]
mod test {
    use crate::analysis::phi::{PhiForge};
    use crate::ir::insert_block::BlockInserter;
    use crate::samples::{get_sample_functions, PRIME};

    #[test]
    fn test_insert() {
        let funcs = get_sample_functions(PRIME);
        let (mut ssa, _) = PhiForge::run(&funcs);
        BlockInserter::run(&mut ssa.functions[0], 3);
        println!("{}", ssa);
    }
}