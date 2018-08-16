extern crate cbindgen;

use std::env;

fn main() {
    if env::var("CARGO_EXPAND_TARGET_DIR").is_err() {
        let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        std::env::set_var("CARGO_EXPAND_TARGET_DIR", "../target/release");
        let mut config: cbindgen::Config = Default::default();
        config.language = cbindgen::Language::C;
        config.parse = cbindgen::ParseConfig::default();
        config.parse.expand = vec!["cmsis-cffi".to_string()];
        match cbindgen::generate_with_config(&crate_dir, config) {
            Ok(k) => {
                k.write_to_file("../target/cmsis.h");
            }
            Err(e) => {
                println!("warning={}", e);
            }
        }
        println!("cargo:rerun-if-changed=cmsis-cffi");
        println!("cargo:rerun-if-changed=target/cmsis.h");
    }
}
