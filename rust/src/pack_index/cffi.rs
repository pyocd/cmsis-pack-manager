use slog::Logger;
use config::Config;
use std::os::raw::c_char;
use std::ffi::CString;
use std::path::PathBuf;
use std::ptr::null;

use super::network::update;

pub struct UpdateReturn(Vec<PathBuf>);

#[no_mangle]
pub extern fn update_pdsc_index() -> *mut UpdateReturn {
    extern crate slog_term;
    extern crate slog_async;
    use slog::Drain;
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let log = Logger::root(drain, o!());
    let conf = Config::new().unwrap();
    let vidx_list = conf.read_vidx_list(&log);
    let updated = update(&conf, vidx_list, &log).unwrap();
    Box::into_raw(Box::new(UpdateReturn(updated)))
}

macro_rules! with_from_raw {
    (let $boxed:ident = $ptr:ident, $block:block) => {
        {
            let $boxed = unsafe {Box::from_raw($ptr)};
            let ret = $block;
            Box::into_raw($boxed);
            ret
        }
    };
    (let mut $boxed:ident = $ptr:ident, $block:block) => {
        {
            let mut $boxed = unsafe {Box::from_raw($ptr)};
            let ret = $block;
            Box::into_raw($boxed);
            ret
        }
    }
}

#[no_mangle]
pub extern fn update_pdsc_index_next(ptr: *mut UpdateReturn) -> *const c_char {
    if ! ptr.is_null() {
        with_from_raw!(let mut boxed = ptr, {
            if let Some(osstr) = boxed.0.pop().map(|p| p.into_os_string()){
                match osstr.to_str() {
                    Some(osstr) => {
                        match CString::new(osstr) {
                            Ok(cstr) => cstr.into_raw(),
                            Err(_) => null()
                        }
                    },
                    None => null()
                }
            } else {
                null()
            }
        })
    } else {
        null()
    }
}

#[no_mangle]
pub extern fn cstring_free(ptr: *mut c_char) {
    if ! ptr.is_null() {
        drop(unsafe {CString::from_raw(ptr)})
    }
}


#[no_mangle]
pub extern fn update_pdsc_index_free(ptr: *mut UpdateReturn) {
    if ! ptr.is_null() {
        drop(unsafe {Box::from_raw(ptr)})
    }
}
