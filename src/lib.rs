#[macro_use]
extern crate nix;

mod gen;

pub use gen::bootparam;
pub mod kvm;
pub mod util;
