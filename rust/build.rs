extern crate cbindgen;

use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut config: cbindgen::Config = Default::default();
    config.language = cbindgen::Language::C;
    match cbindgen::generate_with_config(&crate_dir, config) {
        Ok(k) => {
            k.write_to_file("target/cmsis.h");
        }
        Err(e) => {
            println!("{}", e);
        }
    }
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=target/cmsis.h");
}
