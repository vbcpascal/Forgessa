use depile::ir::{Block, Function, Instr};
use depile::ir::instr::{Branching, BranchKind, InstrExt};
use depile::ir::instr::stripped::{Marker, Operand};
use crate::ssa::{Phi, SSAInterProc, SSAOpd};

pub trait PannableBlock {
    fn panning_forward_fill(&self, offset: usize) -> Self;
}

impl<K: InstrExt> PannableBlock for Block<K>
    where K::Operand: Pannable,
          K::Branching: Pannable,
          K::Marker: Pannable,
          K::InterProc: Pannable,
          K::Extra: Pannable {
    fn panning_forward_fill(&self, offset: usize) -> Self {
        let mut instrs: Vec<Instr<K>> = Vec::new();
        for _ in 0..offset { instrs.push(Instr::Nop); }
        for instr in self.instructions.iter() {
            instrs.push(instr.pan(&|x| x + offset));
        }
        Block { first_index: self.first_index, instructions: instrs.into_boxed_slice() }
    }
}

pub trait Pannable {
    fn pan(&self, f: &impl Fn(usize) -> usize) -> Self;
}

impl Pannable for Operand {
    fn pan(&self, f: &impl Fn(usize) -> usize) -> Self {
        match self {
            Operand::Register(x) => Operand::Register(f(*x)),
            _ => self.clone(),
        }
    }
}

impl Pannable for SSAOpd {
    fn pan(&self, f: &impl Fn(usize) -> usize) -> Self {
        match self {
            SSAOpd::Operand(opd) => SSAOpd::Operand(opd.pan(f)),
            _ => self.clone(),
        }
    }
}

impl<Operand: Pannable> Pannable for BranchKind<Operand> {
    fn pan(&self, f: &impl Fn(usize) -> usize) -> Self {
        match self {
            BranchKind::Unconditional => BranchKind::Unconditional,
            BranchKind::If(opd) => BranchKind::If(opd.pan(f)),
            BranchKind::Unless(opd) => BranchKind::Unless(opd.pan(f)),
        }
    }
}

impl<Operand: Pannable> Pannable for Branching<Operand> {
    fn pan(&self, f: &impl Fn(usize) -> usize) -> Self {
        Branching {
            method: self.method.pan(f),
            dest: self.dest,
        }
    }
}

impl<K: InstrExt> Pannable for Block<K>
    where K::Operand: Pannable,
          K::Branching: Pannable,
          K::Marker: Pannable,
          K::InterProc: Pannable,
          K::Extra: Pannable {
    fn pan(&self, f: &impl Fn(usize) -> usize) -> Self {
        let mut instrs: Vec<Instr<K>> = Vec::new();
        for instr in self.instructions.iter() {
            instrs.push(instr.pan(f));
        }
        Block {
            first_index: f(self.first_index),
            instructions: instrs.into_boxed_slice()
        }
    }
}

pub fn panning_function<K: InstrExt>(func: &Function<K>, first_index: usize) -> (Function<K>, usize)
    where K::Operand: Pannable,
          K::Branching: Pannable,
          K::Marker: Pannable,
          K::InterProc: Pannable,
          K::Extra: Pannable {
    let mut blocks = Vec::new();
    let mut index = first_index;
    for block in func.blocks.iter() {
        let i = block.first_index;
        let block_new = block.pan(&|x| x + index - i);
        blocks.push(block_new);
        index += block.instructions.len();
    }
    (Function {
        parameter_count: func.parameter_count,
        local_var_count: func.local_var_count,
        entry_block: func.entry_block,
        blocks: blocks,
    }, index)
}

impl Pannable for Marker {
    fn pan(&self, _: &impl Fn(usize) -> usize) -> Self { self.clone() }
}

impl Pannable for SSAInterProc {
    fn pan(&self, f: &impl Fn(usize) -> usize) -> Self {
        match self {
            SSAInterProc::PushParam(opd) => SSAInterProc::PushParam(opd.pan(f)),
            _ => self.clone(),
        }
    }
}

impl Pannable for Phi {
    fn pan(&self, f: &impl Fn(usize) -> usize) -> Self {
        let mut res: Vec<SSAOpd> = Vec::new();
        for opd in &self.vars { res.push(opd.pan(f)); }
        Phi { vars: res, blocks: self.blocks.clone(), dest: self.dest.clone() }
    }
}

impl<K: InstrExt> Pannable for Instr<K>
    where K::Operand: Pannable,
          K::Branching: Pannable,
          K::Marker: Pannable,
          K::InterProc: Pannable,
          K::Extra: Pannable {
    fn pan(&self, f: &impl Fn(usize) -> usize) -> Self {
        match self {
            Instr::Binary {op, lhs, rhs} =>
                Instr::Binary {op: op.clone(), lhs: lhs.pan(f), rhs: rhs.pan(f)},
            Instr::Unary {op, operand} =>
                Instr::Unary {op: op.clone(), operand: operand.pan(f)},
            Instr::Branch(b) => Instr::Branch(b.pan(f)),
            Instr::Load(opd) => Instr::Load(opd.pan(f)),
            Instr::Store {data, address} =>
                Instr::Store {data: data.pan(f), address: address.pan(f)},
            Instr::Move {source, dest} =>
                Instr::Move {source: source.pan(f), dest: dest.pan(f)},
            Instr::Read => Instr::Read,
            Instr::Write(opd) => Instr::Write(opd.pan(f)),
            Instr::WriteLn => Instr::WriteLn,
            Instr::InterProc(inter_proc) => Instr::InterProc(inter_proc.pan(f)),
            Instr::Nop => Instr::Nop,
            Instr::Marker(marker) => Instr::Marker(marker.pan(f)),
            Instr::Extra(extra) => Instr::Extra(extra.pan(f)),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ir::converter::block_convert;
    use crate::ir::panning::PannableBlock;
    use crate::samples::{get_sample_functions, PRIME};

    #[test]
    fn test_forward_fill() {
        let funcs = get_sample_functions(PRIME);
        let func = &funcs.functions[0];
        for block in &func.blocks {
            let block = block_convert(block);
            let block_pan = block.panning_forward_fill(5);
            assert_eq!(block.first_index, block_pan.first_index);
            assert_eq!(block.instructions.len() + 5, block_pan.instructions.len());
        }
    }
}