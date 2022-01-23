use depile::ir::Instr;
use crate::ssa::{Phi, SSAFunction, SSAFunctions};

pub struct SSATo3Addr {

}

impl SSATo3Addr {
    pub fn new() -> Self { SSATo3Addr { } }

    pub fn run(funcs: &mut SSAFunctions) {
        let s23 = SSATo3Addr::new();

        for func in &mut funcs.functions {
            s23.remove_phi_func(func);
        }
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
}

mod helper {
    use depile::ir::Instr;
    use crate::ssa::{SSABlock, SSAOpd};

    pub fn push_var_assignment(block: &mut SSABlock, src: &SSAOpd, dst: &SSAOpd) {
        match src {
            SSAOpd::Subscribed(_, i) => if *i < 0 { return; }
            _ => ()
        }
        let stmt = Instr::Move {source: src.clone(), dest: dst.clone()};
        let mut instrs = std::mem::take(&mut block.instructions).into_vec();
        if instrs.is_empty() {
            instrs.push(stmt);
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
}


#[cfg(test)]
mod test {
    use std::io::{ Write, BufWriter };
    use depile::ir::Function;
    use crate::analysis::phi::{find_defs, PhiForge};
    use crate::analysis::ssa_to_aaa::SSATo3Addr;
    use crate::samples::{ALL_SAMPLES, get_sample_functions, PRIME};

    #[test]
    fn test_ssa_to_aaa() {
        let funcs = get_sample_functions(PRIME);
        let mut ssa = PhiForge::run(&funcs);
        let mut func = &mut ssa.functions[0];
        let x = SSATo3Addr { };
        x.remove_phi_func(func);

        println!("{}", func);
    }

    #[test]
    fn test_const_prop() {
        for (i, str) in ALL_SAMPLES.iter().enumerate() {
            let name = crate::samples::samples_str::ALL_SAMPLES[i].to_string().to_lowercase();
            let funcs = get_sample_functions(str);
            let mut ssa = PhiForge::run(&funcs);
            SSATo3Addr::run(&mut ssa);

            let file_path = format!("samples/revert/{}.txt", name);
            let file = std::fs::File::create(file_path).unwrap();
            let mut writer = BufWriter::new(&file);
            write!(&mut writer, "{}", ssa).unwrap();
        }
    }

}
