use slog::Logger;
use config::Config;
use std::path::{PathBuf, Path};
use std::slice::from_raw_parts_mut;

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
pub extern fn update_pdsc_index_maxlen(ptr: *mut UpdateReturn) -> usize {
    if ! ptr.is_null() {
        with_from_raw!(let boxed = ptr, {
            boxed.0.iter().map(|pb|{
                pb.to_str().unwrap_or_default().len()
            }).max()
        }).unwrap_or_default()
    } else {
        0
    }
}

#[no_mangle]
pub extern fn update_pdsc_index_next(
    ptr: *mut UpdateReturn,
    buff: *mut u8,
    buff_size: usize) -> bool {
    if ! ptr.is_null() && ! buff.is_null() {
        with_from_raw!(let mut boxed = ptr, {
            let path = boxed.0.pop();
            if let Some(s) = path.as_ref().map(PathBuf::as_path).and_then(Path::to_str) {
                let len = ::std::cmp::min(buff_size, s.len());
                let (from, _) = s.as_bytes().split_at(len);
                let into = unsafe {from_raw_parts_mut(buff, len)};
                into.copy_from_slice(from);
                true
            } else {
                false
            }
        })
    } else {
        false
    }
}


#[no_mangle]
pub extern fn update_pdsc_index_free(ptr: *mut UpdateReturn) {
    if ! ptr.is_null() {
        let _ = unsafe {Box::from_raw(ptr)};
    }
}
