# ForgeSSA

[![language](https://img.shields.io/badge/language-Rust-red)](https://www.rust-lang.org/)

Build Static Single Assignment (SSA) form, and perform optimizations on SSA. In particular, this project consists of the following parts:

- Build Static Single Assignment (SSA) form from 3-address.
- Perform SSA based constant propagation.
- Perform SSA based loop invariant code motion.
- Translate SSA back to non-SSA 3-address code.

The 3-address intermediate representation is specified by [this lab](https://www.cs.utexas.edu/users/mckinley/380C/labs/lab1.html) and [depile](https://github.com/ruifengx/depile). You can get more information on these pages.

Project for *Advanced Compiler Techniques*.
