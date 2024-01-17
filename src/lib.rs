#[macro_use]
extern crate nix;

#[allow(unused)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
#[allow(clippy::missing_safety_doc)]
pub mod bootparam {
    include!(concat!(env!("OUT_DIR"), "/bootparam.rs"));
}

pub mod constants;
pub mod kvm;
pub mod linux_loader;
pub mod util;
