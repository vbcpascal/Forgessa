
//! SSA Instructions, modifying [`Operand`] and [`Extra`].
//!
//! [`Operand`]: depile::ir::instr::basic::Operand
//! [`Extra`]: depile::ir::instr::basic::Extra

use std::fmt::Formatter;
use depile::analysis::control_flow::{BranchingBehaviour, HasBranchingBehaviour};
use parse_display::{Display, FromStr};
use depile::ir::instr::basic::{Branching, Marker, InterProc, Operand};
use depile::ir::instr::InstrExt;
use crate::ssa::from_basic::func_from_basic;

/// Instruction kind SSA
pub type SSAKind = depile::ir::instr::Kind<SSAOpd, Branching, Marker, InterProc, Phi>;

/// SSA block.
pub type Block = depile::ir::Block<SSAKind>;
pub type Function = depile::ir::Function<SSAKind>;
pub type Functions = depile::ir::Functions<SSAKind>;

/// [`Instr`](depile::ir::Instr)uction with kind "SSA"
pub type SSAInstr = depile::ir::Instr<SSAKind>;

/// Operands to [`SSAInstr`](SSAInstr)uctions.
#[derive(Debug, Display, FromStr, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum SSAOpd {
    /// Basic operands.
    #[display("{0}")]
    Operand(Operand),
    /// Subscribed variable.
    #[display("{0}${1}")]
    Subscribed(String, i64),
}

/// SSA extra instructions.
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Phi {
    vars: Vec<SSAOpd>
}

impl std::fmt::Display for Phi {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "phi").unwrap();
        for var in &self.vars {
            write!(f, " {}", var).unwrap();
        }
        Ok(())
    }
}

impl std::str::FromStr for Phi {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let tokens: Vec<&str> = s.split(" ").collect();
        if tokens[0] != "phi" { return Err(()); }
        let mut vars: Vec<SSAOpd> = Vec::new();
        for tok in tokens[1..].iter() { vars.push(tok.parse().unwrap()); }
        Ok(Phi{vars})
    }
}

impl HasBranchingBehaviour for Phi {
    fn get_branching_behaviour(&self) -> BranchingBehaviour {
        BranchingBehaviour { might_fallthrough: true, alternative_dest: None }
    }
}

pub fn ir_from_basic<K: InstrExt>(funcs: &depile::ir::Functions<K>) -> Functions {
    Functions {
        functions: funcs.functions.iter().map(|x| func_from_basic(x)).collect(),
        entry_function: funcs.entry_function,
    }
}

mod from_basic {
    use depile::ir::instr::{InstrExt, Instr};
    use depile::ir::instr::Instr::Binary;
    use super::{Block, SSAInstr, Function};

    pub fn func_from_basic<K: InstrExt>(func: &depile::ir::Function<K>) -> Function {
        Function {
            parameter_count: func.parameter_count,
            local_var_count: func.local_var_count,
            entry_block: func.entry_block,
            blocks: func.blocks.iter().map(|x| block_from_basic(x)).collect(),
        }
    }

    pub fn block_from_basic<K: InstrExt>(block: &depile::ir::Block<K>) -> Block {
        Block {
            first_index: block.first_index,
            instructions: block.instructions.iter().map(|i| instr_from_basic(i)).collect(),
        }
    }

    pub fn instr_from_basic<K: InstrExt>(instr: &depile::ir::Instr<K>) -> SSAInstr {
        todo!()
    }

}




#[cfg(test)]
mod tests {
    use depile::ir::Blocks;
    use super::{SSAInstr, Phi, SSAOpd, ir_from_basic};

    macro_rules! assert_equiv {
        ($($str: expr => $val: expr),+ $(,)?) => {
            $(
                assert_eq!($val.to_string(), $str);
                assert_eq!($val, $str.parse().unwrap());
            )+
        }
    }

    #[test]
    fn test_operand() {
        use depile::ir::instr::basic::Operand::GP;

        assert_equiv! {
            "GP" => SSAOpd::Operand(GP),
            "i$0" => SSAOpd::Subscribed("i".to_string(), 0),
        }
    }

    #[test]
    fn test_instruction() {
        assert_equiv! {
            "nop" => SSAInstr::Nop,
            "phi i$0 i$1" => SSAInstr::Extra(
                Phi { vars: Vec::from_iter([
                    SSAOpd::Subscribed("i".to_string(), 0),
                    SSAOpd::Subscribed("i".to_string(), 1),
                ])}),
        }
    }

    #[test]
    fn test_read() {
        use depile::ir::program::read_program;
        use crate::samples;

        let program = read_program(samples::GCD).unwrap();
        let blocks = Blocks::try_from(program.as_ref()).unwrap();
        let functions = ir_from_basic(&blocks.functions().unwrap());
        println!("{}", functions);
    }
}
