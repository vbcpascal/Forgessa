use depile::ir::Instr;
use crate::analysis::panning::panning_function;
use crate::analysis::ssa_to_aaa::helper::Substitutable;
use crate::ssa::{Phi, SSAFunction, SSAFunctions, SSAOpd};

pub struct SSATo3Addr { }

impl SSATo3Addr {
    pub fn new() -> Self { SSATo3Addr { } }

    pub fn run(funcs: &mut SSAFunctions, params: &Vec<Vec<String>>) -> Vec<Vec<SSAOpd>> {
        let s23 = SSATo3Addr::new();
        let mut locals = Vec::new();

        for i in 0..params.len() {
            let func = funcs.functions.get_mut(i).unwrap();
            let params = &params[i];
            s23.remove_phi_func(func);
            locals.push(s23.rename_params(func, params));
        }
        s23.flatten(funcs);
        locals
    }

    pub fn remove_phi_func(&self, func: &mut SSAFunction) {
        let mut work_list = Vec::new();
        for block in &mut func.blocks {
            for instr in block.instructions.iter_mut() {
                match instr {
                    Instr::Extra(Phi {vars, blocks, dest}) => {
                        for i in 0..vars.len() {
                            work_list.push((blocks[i], vars[i].clone(), dest.clone()));
                        }
                    }
                    _ => break
                }
                *instr = Instr::Nop;
            }
        }
        for (block_idx, src, dst) in work_list {
            helper::push_var_assignment(&mut func.blocks[block_idx], &src, &dst);
        }
    }

    pub fn rename_params(&self, func: &mut SSAFunction, params: &Vec<String>) -> Vec<SSAOpd> {
        let mut locals = Vec::new();
        for block in &mut func.blocks {
            block.subst(params, &mut locals);
        }
        func.local_var_count = locals.len() as u64;
        locals
    }

    pub fn flatten(&self, funcs: &mut SSAFunctions) {
        let mut index: usize = 3;
        for func in funcs.functions.iter_mut() {
            let res = panning_function(func, index);
            *func = res.0;
            index = res.1;
        }

    }
}

mod helper {
    use depile::ir::Instr;
    use depile::ir::instr::basic::Operand;
    use depile::ir::instr::BranchKind;
    use crate::ssa::{SSABlock, SSAInstr, SSAInterProc, SSAOpd};

    pub fn push_var_assignment(block: &mut SSABlock, src: &SSAOpd, dst: &SSAOpd) {
        match src {
            SSAOpd::Subscribed(_, i) => if *i < 0 { return; }
            _ => ()
        }
        let stmt = Instr::Move {source: src.clone(), dest: dst.clone()};
        let mut instrs = std::mem::take(&mut block.instructions).into_vec();
        if instrs.is_empty() {
            instrs.push(stmt);
            block.instructions = instrs.into_boxed_slice();
            return;
        }

        let last = instrs.pop().unwrap();
        match last {
            Instr::Branch(_) => {
                instrs.push(stmt);
                instrs.push(last);
            }
            _ => {
                instrs.push(last);
                instrs.push(stmt);
            }
        }

        block.instructions = instrs.into_boxed_slice();
    }

    pub trait Substitutable {
        fn subst(&mut self, params: &Vec<String>, locals: &mut Vec<SSAOpd>);
    }

    impl Substitutable for SSABlock {
        fn subst(&mut self, params: &Vec<String>, locals: &mut Vec<SSAOpd>) {
            for instr in self.instructions.iter_mut() {
                instr.subst(params, locals);
            }
        }
    }

    impl Substitutable for SSAInstr {
        fn subst(&mut self, params: &Vec<String>, locals: &mut Vec<SSAOpd>) {
            match self {
                Instr::Binary {op: _, lhs, rhs} =>
                    { lhs.subst(params, locals); rhs.subst(params, locals) }
                Instr::Unary {op: _, operand} =>
                    { operand.subst(params, locals); }
                Instr::Branch(branching) =>
                    match &mut branching.method {
                        BranchKind::If(opd) => opd.subst(params, locals),
                        BranchKind::Unless(opd) => opd.subst(params, locals),
                        _ => ()
                    },
                Instr::Load(opd) =>
                    opd.subst(params, locals),
                Instr::Store {data, address} =>
                    { data.subst(params, locals); address.subst(params, locals); }
                Instr::Move {source, dest} =>
                    { source.subst(params, locals); dest.subst(params, locals); }
                Instr::Read => (),
                Instr::Write(opd) =>
                    opd.subst(params, locals),
                Instr::WriteLn => (),
                Instr::InterProc(interproc) =>
                    match interproc {
                        SSAInterProc::PushParam(opd) => opd.subst(params, locals),
                        _ => (),
                    },
                Instr::Nop => (),
                Instr::Marker(_) => (),
                Instr::Extra(_) => panic!("Error phi node"),
            }
        }
    }

    impl Substitutable for SSAOpd {
        fn subst(&mut self, params: &Vec<String>, locals: &mut Vec<SSAOpd>) {
            match &self.clone() {
                SSAOpd::Subscribed(var, i) => {
                    if params.contains(var) && *i == 0 {
                        let offset: i64 = (params.iter().position(|v| v == var).unwrap()) as i64;
                        *self = SSAOpd::Operand(Operand::Var(var.clone(), offset * 8 + 16));
                        return;
                    }
                    if !locals.contains(self) { locals.push(self.clone()); }
                    let offset: i64 = locals.iter().position(|v| v == self).unwrap() as i64;
                    let var_name = var.clone() + &*i.to_string();
                    *self = SSAOpd::Operand(Operand::Var(var_name, -offset * 8 - 8));
                }
                _ => ()
            }
        }
    }

}


#[cfg(test)]
mod test {
    use std::io::{ Write, BufWriter };
    use crate::analysis::phi::PhiForge;
    use crate::analysis::ssa_to_aaa::SSATo3Addr;
    use crate::samples::{ALL_SAMPLES, get_sample_functions, PRIME};

    #[test]
    fn test_ssa_to_aaa() {
        let funcs = get_sample_functions(PRIME);
        let (mut ssa, params) = PhiForge::run(&funcs);
        SSATo3Addr::run(&mut ssa, &params);

        println!("{}", ssa);
    }

    #[test]
    fn test_const_prop() {
        for (i, str) in ALL_SAMPLES.iter().enumerate() {
            let name = crate::samples::samples_str::ALL_SAMPLES[i].to_string().to_lowercase();
            let funcs = get_sample_functions(str);
            let (mut ssa, params) = PhiForge::run(&funcs);
            SSATo3Addr::run(&mut ssa, &params);

            let file_path = format!("samples/revert/{}.txt", name);
            let file = std::fs::File::create(file_path).unwrap();
            let mut writer = BufWriter::new(&file);
            write!(&mut writer, "{}", ssa).unwrap();
        }
    }

}
