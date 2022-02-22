pub mod ssa;
pub mod samples;
pub mod analysis;
pub mod ir;
pub mod opt;
pub mod cli;

fn main() {
    if let Err(err) = cli::Cli::run() {
        eprintln!("{}", err);
    }
}
