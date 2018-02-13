use slog::Logger;
use std::borrow::Cow;
use std::os::raw::c_char;
use std::ffi::CStr;

use pi::config::ConfigBuilder;
use pack_desc::dump_devices;

#[no_mangle]
pub extern "C" fn dump_pdsc_json(
    pack_store: *const c_char,
    devices_dest: *const c_char,
    boards_dest: *const c_char,
) -> () {
    extern crate slog_term;
    extern crate slog_async;
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
    let dev_dest: Option<Cow<str>> = if !devices_dest.is_null() {
        let fname = unsafe { CStr::from_ptr(devices_dest) }.to_string_lossy();
        Some(fname)
    } else {
        None
    };
    let brd_dest: Option<Cow<str>> = if !boards_dest.is_null() {
        let fname = unsafe { CStr::from_ptr(boards_dest) }.to_string_lossy();
        Some(fname)
    } else {
        None
    };
    let filenames = conf.pack_store.read_dir().unwrap().flat_map(
        |rd| rd.into_iter().map(
            |dirent| dirent.path()
        )).collect::<Vec<_>>();
    if let Err(e) = dump_devices(filenames,
                                 dev_dest.map(|d| d.to_string()),
                                 brd_dest.map(|d| d.to_string()),
                                 &log) {
        println!("pdsc indexing : {:?}", e);
    }
}
