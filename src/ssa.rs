
//! SSA Instructions, modifying [`Operand`] and [`Extra`].
//!
//! [`Operand`]: depile::ir::instr::basic::Operand
//! [`Extra`]: depile::ir::instr::basic::Extra

#![allow(unused)]

use std::fmt::Formatter;
use depile::analysis::control_flow::{BranchingBehaviour, HasBranchingBehaviour};
use depile::ir::instr::basic::Operand;
use parse_display::{Display, FromStr};

/// Instruction kind SSA
pub type SSAKind = depile::ir::instr::Kind<
    SSAOpd,
    depile::ir::instr::Branching<SSAOpd>,
    depile::ir::instr::stripped::Marker,
    depile::ir::instr::basic::InterProc,
    Phi>;

/// SSA block.
pub type SSABlock = depile::ir::Block<SSAKind>;
pub type SSABlocks = depile::ir::Blocks<SSAKind>;
pub type SSAFunction = depile::ir::Function<SSAKind>;
pub type SSAFunctions = depile::ir::Functions<SSAKind>;

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

#[cfg(test)]
mod tests {
    use super::{SSAInstr, Phi, SSAOpd};

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
}
