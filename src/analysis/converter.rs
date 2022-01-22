use depile::ir::Block;
use depile::ir::instr::stripped::Kind;
use crate::ssa::{SSABlock, SSAInstr};

/// Convert a block with kind `Stripped` to `SSAKind` straight forward.
pub fn block_convert(block: &Block<Kind>) -> SSABlock {
    let mut instrs: Vec<SSAInstr> = Vec::new();
    for instr in block.instructions.iter() {
        instrs.push(instr.clone().map_kind(
            convert::map_operand,
            convert::map_branching,
            convert::map_inter_proc,
            std::convert::identity,
            |_| panic!(""),
        ))
    }
    SSABlock { first_index: block.first_index, instructions: instrs.into_boxed_slice() }
}


///
mod convert {
    use depile::ir::instr::{Branching, BranchKind};
    use depile::ir::instr::stripped::{InterProc, Operand};
    use crate::ssa::{SSAInterProc, SSAOpd};

    pub fn map_operand(opd: Operand) -> SSAOpd {
        SSAOpd::Operand(opd)
    }

    pub fn map_branch_kind(brk: BranchKind<Operand>) -> BranchKind<SSAOpd> {
        match brk {
            BranchKind::Unconditional => BranchKind::Unconditional,
            BranchKind::If(opd) => BranchKind::If(map_operand(opd)),
            BranchKind::Unless(opd) => BranchKind::Unless(map_operand(opd)),
        }
    }

    pub fn map_branching(branching: Branching<Operand>) -> Branching<SSAOpd> {
        Branching {
            method: map_branch_kind(branching.method),
            dest: branching.dest,
        }
    }

    pub fn map_inter_proc(inter: InterProc) -> SSAInterProc {
        match inter {
            InterProc::PushParam(opd) => SSAInterProc::PushParam(map_operand(opd)),
            InterProc::Call {dest} => SSAInterProc::Call {dest}
        }
    }
}

#[cfg(test)]
mod test {
    use depile::ir::Function;
    use crate::analysis::converter::block_convert;
    use crate::samples::{get_sample_functions, PRIME};

    #[test]
    fn test_convert() {
        let funcs = get_sample_functions(PRIME);
        let func: &Function = &funcs.functions[0];
        let block = block_convert(&func.blocks[0]);
        println!("{}", block);
    }
}
