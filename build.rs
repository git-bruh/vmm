use std::{env, path::PathBuf};

fn main() {
    let bootparam = PathBuf::from(env::var("OUT_DIR").unwrap()).join("bootparam.rs");
    bindgen::builder()
        .header_contents("bootparam.h", "#include <asm/bootparam.h>")
        .derive_default(true)
        .generate()
        .expect("unable to generate bindings for bootparam.h")
        .write_to_file(bootparam)
        .expect("unable to write bindings for bootparam.h");
}
