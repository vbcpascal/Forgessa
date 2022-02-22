
use std::path::PathBuf;
use thiserror::Error;
use displaydoc::Display as DisplayDoc;
use parse_display::{Display, FromStr};
use clap::{ArgEnum, Parser};

use depile::ir::{block, function, Blocks};
use depile::ir::program::{self, display_program, read_program};
use crate::analysis::phi::PhiForge;
use crate::ir::converter::functions_revert;
use crate::ir::ssa_to_aaa::SSATo3Addr;
use crate::opt::loop_invariant::LoopInVariant;

/// Entry to the command line interface.
#[derive(Parser)]
#[clap(author, version, about)]
pub struct Cli {
    /// The input three-address code source file.
    #[clap(parse(from_os_str))]
    input: PathBuf,
    /// Output format.
    #[clap(short, long, arg_enum, default_value_t = Format::SSA)]
    target: Format,
    /// Optimizations.
    #[clap(short, long, arg_enum, default_value_t = OptOption::None)]
    opt: OptOption,
}

/// Supported target formats.
#[derive(Debug, Display, FromStr, ArgEnum, Copy, Clone, Eq, PartialEq)]
#[display(style = "snake_case")]
pub enum Format {
    /// Print out the input file unchanged (disregarding whitespaces).
    Raw,
    /// Partition the input file as basic blocks, and group the basic blocks into functions.
    Functions,
    /// Static single assignment.
    SSA,
    /// Stripped 3-address after converting to SSA.
    Recovered,
    /// Flat 3-address after converting to SSA.
    Flatten,
}

/// Supported optimizations.
#[derive(Debug, Display, FromStr, ArgEnum, Copy, Clone, Eq, PartialEq)]
#[display(style = "snake_case")]
pub enum OptOption {
    /// No optimization.
    None,
    /// Constant propagation.
    ConstProp,
    /// Loop invariant code motion.
    LoopInv,
    /// All the optimizations.
    All,
}

/// All kinds of errors that might happen during command line execution.
#[derive(Debug, DisplayDoc, Error)]
pub enum Error {
    /// "errors" from [`clap`], including requests such as `--version` or `--help`.
    #[displaydoc("{0}")]
    InvalidArguments(#[from] clap::Error),
    /// parse error: {0}
    InvalidInput(#[from] program::ParseError),
    /// failed to parse into basic blocks: {0}
    MalformedBlocks(#[from] block::Error),
    /// failed to group into functions: {0}
    MalformedFunctions(#[from] function::Error),
    /// failed to resolve function call instructions: {0}
    CannotResolveFunctionCall(#[from] function::ResolveError),
    /// failed to read file: {0}
    Io(#[from] std::io::Error),
    /// cannot format the output: {0}
    CannotFormat(#[from] std::fmt::Error),
}

/// Result type for the command line interface.
pub type Result = std::result::Result<(), Error>;

impl Cli {
    /// Run the command line interface.
    pub fn run() -> Result {
        let options: Cli = Cli::try_parse()?;
        let contents = std::fs::read_to_string(&options.input)?;
        let program = read_program(&contents)?;

        match options.target {
            Format::Raw => {
                println!("{}", display_program(&program)?);
                return Ok(());
            }
            Format::Functions => {
                let blocks = Blocks::try_from(program.as_ref())?;
                let functions = blocks.functions()?;
                println!("{}", functions);
                return Ok(());
            }
            _ => ()
        }

        let blocks = Blocks::try_from(program.as_ref())?;
        let functions = blocks.functions()?;
        let (mut ssa, params) = PhiForge::run(&functions);

        match options.opt {
            OptOption::ConstProp => {
                let reports = crate::opt::const_prop::ConstProp::run(&mut ssa);
                println!("Report of constant propagation: ");
                for r in reports { println!("{}", r); }
            }
            OptOption::LoopInv => {
                let reports = LoopInVariant::run(&mut ssa);
                println!("Report of loop invariant: ");
                for r in reports { println!("{}", r); }
            }
            OptOption::All => {
                let reports = crate::opt::const_prop::ConstProp::run(&mut ssa);
                println!("Report of constant propagation: ");
                for r in reports { println!("{}", r); }
                let reports = LoopInVariant::run(&mut ssa);
                println!("Report of loop invariant: ");
                for r in reports { println!("{}", r); }
            }
            _ => ()
        }

        match options.target {
            Format::SSA => {
                println!("{}", ssa)
            }
            Format::Recovered => {
                SSATo3Addr::run(&mut ssa, &params);
                println!("{}", ssa)
            }
            Format::Flatten => {
                SSATo3Addr::run(&mut ssa, &params);
                let funcs = functions_revert(&ssa);
                let new_prog = funcs.destruct().flatten();
                println!("{}", display_program(&new_prog)?)
            }
            _ => ()
        }
        Ok(())
    }
}
