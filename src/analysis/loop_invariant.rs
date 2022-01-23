use std::collections::BTreeSet;
use std::fmt::Display;
use depile::ir::Instr;
use depile::ir::instr::basic::Operand;
use smallvec::alloc::fmt::Formatter;
use crate::analysis::loop_invariant::helper::Substitutable;
use crate::analysis::natural_loop::NaturalLoop;
use crate::ir::panning::panning_function;
use crate::ir::insert_block::BlockInserter;
use crate::ssa::{SSABlock, SSAFunction, SSAFunctions, SSAInstr, SSAOpd};

pub struct LoopInvariantReport {
    pub instr_idx: usize,
    pub opt_count: usize,
    pub instructions: Vec<(SSAInstr, usize)>,
}

impl Display for LoopInvariantReport {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "  Function: {}", self.instr_idx)?;
        writeln!(f, "  Number of statement hoisted: {}", self.opt_count)?;
        for (instr, id) in &self.instructions {
            writeln!(f, "  {}: {}", id, instr)?;
        }
        Ok(())
    }
}

pub struct LoopInVariant {
    pub counter: usize,
    pub opt_instr: Vec<(SSAInstr, usize)>,
}

impl LoopInVariant {
    pub fn new() -> Self { LoopInVariant { counter: 0, opt_instr: Vec::new() } }

    pub fn run(funcs: &mut SSAFunctions) -> Vec<LoopInvariantReport> {
        let mut reports = Vec::new();
        for func in &mut funcs.functions {
            reports.push(LoopInVariant::run_func(func)) ;
        }
        reports
    }

    pub fn run_func(func: &mut SSAFunction) -> LoopInvariantReport {
        let mut lv = LoopInVariant::new();
        let loops = NaturalLoop::compute_loops(func);

        for nl in &loops { BlockInserter::run(func, nl.root); }
        // Re-compute the natural loop for inserting blocks.
        let loops = NaturalLoop::compute_loops(func);
        let mut changed = true;

        while changed {
            changed = false;

            // For each natural loop,
            for nl in &loops {
                let root = nl.root;
                let nodes = &nl.nodes;

                // Get the definitions in these blocks.
                let mut defs = BTreeSet::new();
                for n in nodes {
                    helper::get_defs(&func.blocks[*n], &mut defs);
                }

                // Find invariant instruction.
                let mut res: Option<(SSAInstr, usize)> = None;
                for n in nodes {
                    let mut block = &mut func.blocks[*n];
                    res = lv.invariant_block(&mut block, &defs);
                    if res.is_some() { break; }
                }
                if res.is_none() { continue; }

                // Substitution
                lv.counter += 1;
                changed = true;
                let (instr, instr_idx) = res.unwrap();
                lv.opt_instr.push((instr.clone(), instr_idx.clone()));
                let src = SSAOpd::Operand(Operand::Register(instr_idx));
                let tgt = lv.compute_target_opd(func, root);
                for block in &mut func.blocks {
                    block.subst(&src, &tgt);
                }
                lv.push_invariant_instr(func, instr, root);
                break;
            }
        }

        LoopInvariantReport {
            instr_idx: func.blocks[0].first_index,
            opt_count: lv.counter,
            instructions: lv.opt_instr,
        }
    }

    fn push_invariant_instr(&self, func: &mut SSAFunction, instr: SSAInstr, root: usize) {
        let block = &mut func.blocks[root - 1];
        let mut instrs = std::mem::take(&mut block.instructions).into_vec();
        instrs.push(instr);
        block.instructions = instrs.into_boxed_slice();
        *func = panning_function(func, func.blocks[0].first_index).0;
    }

    /// Compute the index of target instruction.
    fn compute_target_opd(&self, func: &SSAFunction, root: usize) -> SSAOpd {
        let block = &func.blocks[root - 1];
        let target_idx = block.first_index + block.instructions.len();
        // This is dirty for the following panning.
        SSAOpd::Operand(Operand::Register(target_idx - 1))
    }

    /// Find invariant code in a `block` according to `defs`.
    fn invariant_block(&self, block: &mut SSABlock, defs: &BTreeSet<SSAOpd>) -> Option<(SSAInstr, usize)> {
        let mut instr_index = block.first_index;
        for instr in block.instructions.iter_mut() {
            if self.check_invariant_instr(instr, &defs) {
                let instr_ = instr.clone();
                *instr = Instr::Nop;
                return Some((instr_, instr_index));
            }
            instr_index += 1;
        }
        None
    }

    /// Check whether an `instr`uction is invariant according to `defs`.
    fn check_invariant_instr(&self, instr: &SSAInstr, defs: &BTreeSet<SSAOpd>) -> bool {
        match instr {
            Instr::Binary {op: _, lhs, rhs} =>
                !defs.contains(lhs) && !defs.contains(rhs),
            Instr::Unary {op: _, operand} =>
                !defs.contains(operand),
            Instr::Load(opd) =>
                !defs.contains(opd),
            Instr::Store {data, address} =>
                !defs.contains(data) && !defs.contains(address),
            Instr::Move {source, dest} =>
                !defs.contains(source) && !defs.contains(dest),
            _ => false
        }
    }
}

mod helper {
    use std::collections::BTreeSet;
    use depile::ir::Instr;
    use depile::ir::instr::basic::Operand;
    use depile::ir::instr::BranchKind;
    use crate::ssa::{Phi, SSABlock, SSAInstr, SSAInterProc, SSAOpd};

    pub fn get_defs(block: &SSABlock, defs: &mut BTreeSet<SSAOpd>) {
        let mut instr_index = block.first_index;
        for instr in block.instructions.iter() {
            defs.insert(SSAOpd::Operand(Operand::Register(instr_index)));
            match instr {
                SSAInstr::Move {source: _, dest} =>
                    { defs.insert(dest.clone()); }
                SSAInstr::Extra(Phi {vars: _, blocks: _, dest}) =>
                    { defs.insert(dest.clone()); }
                _ => (),
            }
            instr_index += 1;
        }
    }


    pub trait Substitutable {
        fn subst(&mut self, origin: &SSAOpd, new: &SSAOpd);
    }

    impl Substitutable for SSABlock {
        fn subst(&mut self, origin: &SSAOpd, new: &SSAOpd) {
            for instr in self.instructions.iter_mut() {
                instr.subst(origin, new);
            }
        }
    }

    impl Substitutable for SSAInstr {
        fn subst(&mut self, origin: &SSAOpd, new: &SSAOpd) {
            match self {
                Instr::Binary {op: _, lhs, rhs} =>
                    { lhs.subst(origin, new); rhs.subst(origin, new) }
                Instr::Unary {op: _, operand} =>
                    { operand.subst(origin, new); }
                Instr::Branch(branching) =>
                    match &mut branching.method {
                        BranchKind::If(opd) => opd.subst(origin, new),
                        BranchKind::Unless(opd) => opd.subst(origin, new),
                        _ => ()
                    },
                Instr::Load(_) => (),
                Instr::Store {data, address} =>
                    { data.subst(origin, new); address.subst(origin, new); }
                Instr::Move {source, dest} =>
                    { source.subst(origin, new); dest.subst(origin, new); }
                Instr::Read => (),
                Instr::Write(opd) =>
                    opd.subst(origin, new),
                Instr::WriteLn => (),
                Instr::InterProc(interproc) =>
                    match interproc {
                        SSAInterProc::PushParam(opd) => opd.subst(origin, new),
                        _ => (),
                    },
                Instr::Nop => (),
                Instr::Marker(_) => (),
                Instr::Extra(_) => (),
            }
        }
    }

    impl Substitutable for SSAOpd {
        fn subst(&mut self, origin: &SSAOpd, new: &SSAOpd) {
            if self == origin {*self = new.clone();}
        }
    }
}

#[cfg(test)]
mod test {
    use std::io::Write;
    use std::io::BufWriter;
    use crate::analysis::loop_invariant::LoopInVariant;
    use crate::analysis::phi::PhiForge;
    use crate::samples::{ALL_SAMPLES, COLLATZ, get_sample_functions};

    #[test]
    fn test_loop() {
        let funcs = get_sample_functions(COLLATZ);
        let (mut ssa, _) = PhiForge::run(&funcs);
        LoopInVariant::run(&mut ssa);
        println!("{}", ssa);
    }

    #[test]
    fn test_samples_loop() {
        for (i, str) in ALL_SAMPLES.iter().enumerate() {
            let name = crate::samples::samples_str::ALL_SAMPLES[i].to_string().to_lowercase();
            if name == "regslarge" { continue; }
            let funcs = get_sample_functions(str);
            let (mut ssa, _) = PhiForge::run(&funcs);
            let reports = LoopInVariant::run(&mut ssa);

            let file_path = format!("samples/loop/{}.txt", name);
            let file = std::fs::File::create(file_path).unwrap();
            let mut writer = BufWriter::new(&file);
            writeln!(&mut writer, "Report of {}:", name).expect("Error");
            for r in reports { writeln!(&mut writer, "{}", r).expect("error"); }
            write!(&mut writer, "{}", ssa).unwrap();
        }
    }
}