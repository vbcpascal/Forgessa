use depile::ir::instr::HasOperand;
use depile::ir::instr::stripped::{Function, Operand};
use smallvec::SmallVec;

pub fn scan_parameters(func: &Function) -> Vec<String> {
    let count = func.parameter_count;
    let mut params: Vec<String> = Vec::new();
    params.resize(usize::try_from(count).unwrap(), String::from("<unknown>"));
    for block in &func.blocks {
        for instr in block.instructions.iter() {
            let opds: SmallVec<[&Operand;2]> = instr.get_operands();
            for opd in opds {
                match opd {
                    Operand::Var(var, x) => if *x > 0 {
                        *params.get_mut(usize::try_from(x / 8 - 2).unwrap()).unwrap() = var.clone();
                    }
                    _ => ()
                }
            }
        }
    }
    params
}

#[cfg(test)]
mod test {
    use crate::ir::params::scan_parameters;
    use crate::samples::{GCD, get_sample_functions};

    #[test]
    fn test_scan() {
        let funcs = get_sample_functions(GCD);
        let func = &funcs.functions[0];
        let params = scan_parameters(func);
        println!("{:?}", params);
    }
}
