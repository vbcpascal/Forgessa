
#![allow(unused)]

use depile::ir::{Blocks, Functions, Program};
use depile::ir::program::read_program;

macro_rules! count {
    () => { 0 };
    ($x: tt $(, $xs: tt)*) => { 1 + count!($($xs),*) }
}

macro_rules! include_samples {
    ($($name: ident),+ $(,)?) => {
        $(
            pub const $name: &str = include_str!(concat!("../dependencies/depile/src/samples/", stringify!($name), ".txt"));
        )+
        pub const ALL_SAMPLES: [&str; count!($($name),+)] = [$($name),+];

        pub mod samples_str {
            $( pub const $name: &str = stringify!($name); )+
            pub const ALL_SAMPLES: [&str; count!($($name),+)] = [$($name),+];
        }
    }
}

include_samples! {
    COLLATZ,
    GCD,
    HANOIFIBFAC,
    LOOP,
    MMM,
    PRIME,
    REGSLARGE,
    SIEVE,
    SORT,
    STRUCT,
}

pub fn get_sample_functions(str: &str) -> Functions {
    let program: Box<Program> = read_program(str).unwrap();
    let blocks: Blocks<depile::ir::instr::basic::Kind> = Blocks::try_from(program.as_ref()).unwrap();
    blocks.functions().unwrap()
}
