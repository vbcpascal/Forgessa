use depile::ir::{Block, Functions};
use depile::ir::instr::stripped::{Function, Kind};
use crate::ssa::{SSABlock, SSAFunction, SSAFunctions, SSAInstr};

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

pub fn functions_revert(funcs: &SSAFunctions) -> Functions<Kind> {
    let mut funcs_ = Vec::new();
    for func in &funcs.functions {
        funcs_.push(func_revert(func));
    }
    Functions {
        functions: funcs_,
        entry_function: funcs.entry_function,
    }
}

pub fn func_revert(func: &SSAFunction) -> Function {
    let mut blocks = Vec::new();
    for block in &func.blocks {
        blocks.push(block_revert(block));
    }
    Function {
        parameter_count: func.parameter_count,
        local_var_count: func.local_var_count,
        entry_block: func.entry_block,
        blocks: blocks,
    }
}

pub fn block_revert(block: &SSABlock) -> Block<Kind> {
    let mut instrs = Vec::new();
    for instr in block.instructions.iter() {
        instrs.push(instr.clone().map_kind(
            revert::map_operand,
            revert::map_branching,
            revert::map_inter_proc,
            std::convert::identity,
            |_| panic!(""),
        ))
    }
    Block { first_index: block.first_index, instructions: instrs.into_boxed_slice() }
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

mod revert {
    use depile::ir::instr::{Branching, BranchKind};
    use depile::ir::instr::stripped::{InterProc, Operand};
    use crate::ssa::{SSAInterProc, SSAOpd};

    pub fn map_operand(opd: SSAOpd) -> Operand {
        match opd {
            SSAOpd::Operand(opd) => opd,
            _ => panic!(""),
        }
    }

    pub fn map_branch_kind(brk: BranchKind<SSAOpd>) -> BranchKind<Operand> {
        match brk {
            BranchKind::Unconditional => BranchKind::Unconditional,
            BranchKind::If(opd) => BranchKind::If(map_operand(opd)),
            BranchKind::Unless(opd) => BranchKind::Unless(map_operand(opd)),
        }
    }

    pub fn map_branching(branching: Branching<SSAOpd>) -> Branching<Operand> {
        Branching {
            method: map_branch_kind(branching.method),
            dest: branching.dest,
        }
    }

    pub fn map_inter_proc(inter: SSAInterProc) -> InterProc {
        match inter {
            SSAInterProc::PushParam(opd) => InterProc::PushParam(map_operand(opd)),
            SSAInterProc::Call {dest} => InterProc::Call {dest}
        }
    }
}

#[cfg(test)]
mod test {
    use depile::ir::Function;
    use crate::ir::converter::block_convert;
    use crate::samples::{get_sample_functions, PRIME};

    #[test]
    fn test_convert() {
        let funcs = get_sample_functions(PRIME);
        let func: &Function = &funcs.functions[0];
        let block = block_convert(&func.blocks[0]);
        println!("{}", block);
    }
}
