// #[macro_use]
// extern crate slog;
// extern crate slog_term;
// extern crate slog_async;
// extern crate pack_index as pi;
// extern crate pdsc as pack_desc;
// extern crate utils as cmsis_utils;
// extern crate failure;

macro_rules! with_from_raw {
    (let $boxed:ident = $ptr:ident, $block:block) => {
        {
            #[allow(unused_unsafe)]
            let $boxed = unsafe {Box::from_raw($ptr)};
            let ret = $block;
            Box::into_raw($boxed);
            ret
        }
    };
    (let mut $boxed:ident = $ptr:ident, $block:block) => {
        {
            #[allow(unused_unsafe)]
            let mut $boxed = unsafe{Box::from_raw($ptr)};
            let ret = $block;
            Box::into_raw($boxed);
            ret
        }
    }
}
#[macro_use]
pub mod utils;

pub mod config;
pub mod pack_index;
pub mod pdsc;
pub mod pack;



