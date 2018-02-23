use slog::Logger;
use std::os::raw::c_char;
use std::ffi::{CStr, CString};
use std::path::PathBuf;
use std::ptr::{null, null_mut};

use cmsis_update::update;
use pi::config::ConfigBuilder;

pub struct UpdateReturn(Vec<PathBuf>);

impl UpdateReturn {
    pub fn iter(&self) -> impl Iterator<Item = &PathBuf> {
        self.0.iter()
    }
}

#[no_mangle]
pub extern "C" fn update_pdsc_index(
    pack_store: *const c_char,
    vidx_list: *const c_char,
) -> *mut UpdateReturn {
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
    let conf_bld = if !vidx_list.is_null() {
        let vlist = unsafe { CStr::from_ptr(vidx_list) }.to_string_lossy();
        conf_bld.with_vidx_list(vlist.into_owned())
    } else {
        conf_bld
    };
    let conf = match conf_bld.build() {
        Ok(c) => c,
        Err(e) => {
            println!("config: {:?}", e);
            return null_mut();
        }
    };
    let vidx_list = conf.read_vidx_list(&log);
    match update(&conf, vidx_list, &log, true) {
        Ok(updated) => Box::into_raw(Box::new(UpdateReturn(updated))),
        Err(e) => {
            println!("pack indexing : {:?}", e);
            null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn update_pdsc_index_next(ptr: *mut UpdateReturn) -> *const c_char {
    if !ptr.is_null() {
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
pub extern "C" fn cstring_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(unsafe { CString::from_raw(ptr) })
    }
}

#[no_mangle]
pub extern "C" fn update_pdsc_index_free(ptr: *mut UpdateReturn) {
    if !ptr.is_null() {
        drop(unsafe { Box::from_raw(ptr) })
    }
}
