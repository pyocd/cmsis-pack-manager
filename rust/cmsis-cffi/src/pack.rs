use slog::Logger;
use std::os::raw::c_char;
use std::ffi::{CStr};

use cmsis_update::install;
use pi::config::ConfigBuilder;

use pdsc::ParsedPacks;


#[no_mangle]
pub extern "C" fn update_packs(pack_store: *const c_char, parsed_packs: *mut ParsedPacks){
    extern crate slog_term;
    extern crate slog_async;
    if !parsed_packs.is_null() {
        use slog::Drain;
        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::FullFormat::new(decorator).build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let log = Logger::root(drain, o!());
        let conf_bld = ConfigBuilder::new();
        let conf_bld = if !pack_store.is_null() {
            let pstore = unsafe { CStr::from_ptr(pack_store) }.to_string_lossy();
            conf_bld.with_pack_store(pstore.into_owned())
        } else {
            conf_bld
        };
        let conf = match conf_bld.build() {
            Ok(c) => c,
            Err(e) => {
                println!("config: {:?}", e);
                return;
            }
        };
        with_from_raw!(let packs = parsed_packs, {
            {
                install(&conf, packs.iter(), &log, true);
            }
        })
    }
}
