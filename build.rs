use std::process::Command;

const ASM: &str = "asm/write_serial.S";
const OUT: &str = "write_serial";

fn main() {
    println!("cargo:rerun-if-changed={ASM}");

    Command::new("nasm")
        .args([
            "-f",
            "bin",
            "-o",
            &format!(
                "{}/{OUT}",
                std::env::var("OUT_DIR").expect("'OUT_DIR' not set!")
            ),
            ASM,
        ])
        .output()
        .expect("failed to compile asm '{ASM}'");
}
