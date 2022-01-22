use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use depile::ir::Instr;
use depile::ir::instr::basic::Operand::Const;
use depile::ir::instr::stripped::Operand;
use crate::ssa::{Phi, SSABlock, SSAFunction, SSAFunctions, SSAInstr, SSAOpd};

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct ConstPropReport {
    pub instr_idx: usize,
    pub opt_count: usize,
}

impl Display for ConstPropReport {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "  Function: {}", self.instr_idx)?;
        writeln!(f, "  Number of constants propagated: {}", self.opt_count)
    }
}

pub struct ConstProp {
    pub count: usize,
    pub const_elements: BTreeMap<SSAOpd, SSAOpd>,
}

impl ConstProp {
    pub fn run(funcs: &mut SSAFunctions) -> Vec<ConstPropReport> {
        let mut reports = Vec::new();
        for func in funcs.functions.iter_mut() {
            reports.push(ConstProp::run_func(func)) ;
        }
        reports
    }

    pub fn run_func(func: &mut SSAFunction) -> ConstPropReport {
        let mut cp = ConstProp::new();
        while func.subst(&mut cp) { };
        ConstPropReport {
            instr_idx: func.blocks[0].first_index,
            opt_count: cp.count,
        }
    }

    pub fn new() -> Self {
        ConstProp {
            count: 0,
            const_elements: BTreeMap::new(),
        }
    }

    pub fn check_subst(&mut self, opd: &mut SSAOpd) -> bool {
        if self.constains(opd) { self.subst(opd); true }
        else { false }
    }

    pub fn constains(&self, opd: &SSAOpd) -> bool {
        self.const_elements.contains_key(opd)
    }

    pub fn subst(&mut self, opd: &mut SSAOpd) {
        self.count += 1;
        *opd = self.const_elements.get(opd).unwrap().clone();
    }

    pub fn insert(&mut self, opd: &SSAOpd, opd_const: &SSAOpd) {
        self.const_elements.insert(opd.clone(), opd_const.clone());
    }

}

pub trait Substitutable {
    fn subst(&mut self, cp: &mut ConstProp) -> bool;
}

pub fn as_constant(opd: &SSAOpd) -> Option<&SSAOpd> {
    match opd {
        SSAOpd::Operand(Operand::Const(_)) => Some(opd),
        _ => None,
    }
}

impl Substitutable for SSAFunction {
    fn subst(&mut self, cp: &mut ConstProp) -> bool {
        let mut changed = false;
        for block in self.blocks.iter_mut() {
            changed |= block.subst(cp);
        }
        changed
    }
}

impl Substitutable for SSABlock {
    fn subst(&mut self, cp: &mut ConstProp) -> bool {
        let mut changed = false;
        for instr in self.instructions.iter_mut() {
            changed |= instr.subst(cp);
        }
        changed
    }
}

impl Substitutable for SSAInstr {
    fn subst(&mut self, cp: &mut ConstProp) -> bool {
        match self {
            Instr::Binary {op, lhs, rhs} =>
                cp.check_subst(lhs) || cp.check_subst(rhs),
            Instr::Unary {op, operand} =>
                cp.check_subst(operand),
            Instr::Branch(branching) => false,
            Instr::Load(opd) => false,
            Instr::Store {data, address } =>
                cp.check_subst(data),
            Instr::Move {source, dest} => {
                let changed = cp.check_subst(source);
                match as_constant(source) {
                    Some(opd) => cp.insert(dest, source),
                    None => (),
                };
                changed
            }
            Instr::Read => false,
            Instr::Write(opd) => false,
            Instr::WriteLn => false,
            Instr::InterProc(interproc) => false,
            Instr::Nop => false,
            Instr::Marker(marker) => false,
            Instr::Extra(Phi {vars}) => {
                let mut changes = false;
                for var in vars { changes |= cp.check_subst(var); }
                changes
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::io::{BufWriter, Write};
    use crate::analysis::const_prop::ConstProp;
    use crate::analysis::phi::PhiForge;
    use crate::samples::{ALL_SAMPLES, get_sample_functions};

    #[test]
    fn test_const_prop() {
        for (i, str) in ALL_SAMPLES.iter().enumerate() {
            let name = crate::samples::samples_str::ALL_SAMPLES[i].to_string().to_lowercase();
            let funcs = get_sample_functions(str);
            let mut ssa = PhiForge::run(&funcs);
            let reports = ConstProp::run(&mut ssa);


            let mut file_path = format!("samples/const_prop/{}.txt", name);
            let file = std::fs::File::create(file_path).unwrap();
            let mut writer = BufWriter::new(&file);
            writeln!(&mut writer, "Report of {}:", name);
            for r in reports { writeln!(&mut writer, "{}", r); }
            write!(&mut writer, "{}", ssa).unwrap();
        }
    }
}