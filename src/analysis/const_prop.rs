use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use depile::ir::Instr;
use depile::ir::instr::basic::Operand::Const;
use depile::ir::instr::BranchKind;
use depile::ir::instr::stripped::Operand;
use crate::ssa::{Phi, SSABlock, SSAFunction, SSAFunctions, SSAInstr, SSAInterProc, SSAOpd};

/// Reports the performance of constant propagation.
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
        let instr_idx = self.first_index;
        for instr in self.instructions.iter_mut() {
            changed |= IdxInstr { idx: instr_idx, instr: instr }.subst(cp);
        }
        changed
    }
}

pub struct IdxInstr<'a> {
    pub idx: usize,
    pub instr: &'a mut SSAInstr,
}

impl Substitutable for IdxInstr<'_> {
    fn subst(&mut self, cp: &mut ConstProp) -> bool {
        let instr = &mut self.instr;
        match instr {
            Instr::Binary {op: _, lhs, rhs} =>
                cp.check_subst(lhs) || cp.check_subst(rhs),
            Instr::Unary {op: _, operand} =>
                cp.check_subst(operand),
            Instr::Branch(branching) =>
                match &mut branching.method {
                    BranchKind::If(opd) => cp.check_subst(opd),
                    BranchKind::Unless(opd) => cp.check_subst(opd),
                    _ => false
                },
            Instr::Load(opd) =>
                cp.check_subst(opd),
            Instr::Store {data, address} =>
                cp.check_subst(data) || cp.check_subst(address),
            Instr::Move {source, dest} => {
                let mut changed = cp.check_subst(source);
                match as_constant(source) {
                    Some(_) => {
                        cp.insert(dest, source);
                        **instr = Instr::Nop;
                        changed = true;
                    },
                    None => (),
                };
                changed
            }
            Instr::Read => false,
            Instr::Write(opd) =>
                cp.check_subst(opd),
            Instr::WriteLn => false,
            Instr::InterProc(interproc) =>
                match interproc {
                    SSAInterProc::PushParam(opd) => cp.check_subst(opd),
                    _ => false,
                },
            Instr::Nop => false,
            Instr::Marker(_) => false,
            Instr::Extra(Phi {vars, blocks: _, dest}) => {
                let mut changed = false;
                for var in vars.iter_mut() { changed |= cp.check_subst(var); }

                match check_vars_in_phi(vars) {
                    Some(opd) => {
                        cp.insert(dest, &opd);
                        **instr = Instr::Nop;
                        changed = true;
                    },
                    None => ()
                }
                changed
            }
        }
    }
}

pub fn as_constant(opd: &SSAOpd) -> Option<&SSAOpd> {
    match opd {
        SSAOpd::Operand(Operand::Const(_)) => Some(opd),
        _ => None,
    }
}

pub fn check_vars_in_phi(vars: &Vec<SSAOpd>) -> Option<SSAOpd> {
    let mut curr: Option<i64> = None;
    for var in vars {
        match var {
            SSAOpd::Operand(Const(i)) => {
                if curr.is_none() { curr = Some(*i); }
                else if curr.is_some() && curr == Some(*i) { }
                else { return None; }
            }
            SSAOpd::Subscribed(_, index) => {
                if *index >= 0 { return None }
                else { continue; }
           }
            _ => panic!("error phi")
        }
    }
    Some(SSAOpd::Operand(Const(curr.unwrap())))
}

#[cfg(test)]
mod test {
    use std::io::{BufWriter, Write};
    use depile::ir::instr::basic::Operand::Const;
    use crate::analysis::const_prop::{check_vars_in_phi, ConstProp};
    use crate::analysis::phi::PhiForge;
    use crate::samples::{ALL_SAMPLES, get_sample_functions};
    use crate::ssa::SSAOpd;

    #[test]
    fn test_const_prop() {
        for (i, str) in ALL_SAMPLES.iter().enumerate() {
            let name = crate::samples::samples_str::ALL_SAMPLES[i].to_string().to_lowercase();
            let funcs = get_sample_functions(str);
            let (mut ssa, _) = PhiForge::run(&funcs);
            let reports = ConstProp::run(&mut ssa);

            let file_path = format!("samples/const_prop/{}.txt", name);
            let file = std::fs::File::create(file_path).unwrap();
            let mut writer = BufWriter::new(&file);
            writeln!(&mut writer, "Report of {}:", name).expect("Error");
            for r in reports { writeln!(&mut writer, "{}", r).expect("error"); }
            write!(&mut writer, "{}", ssa).unwrap();
        }
    }

    #[test]
    fn test_check_vars_phi() {
        let v = SSAOpd::Operand(Const(4));
        let mut vars = Vec::new();
        for _ in 0..3 { vars.push(v.clone()); }
        assert!(check_vars_in_phi(&vars).is_some());

        vars.push(SSAOpd::Subscribed(String::from("v"), -1));
        assert!(check_vars_in_phi(&vars).is_some());
    }
}
