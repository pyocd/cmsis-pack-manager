#![feature(conservative_impl_trait)]

#[macro_use]
extern crate slog;
extern crate slog_term;
extern crate slog_async;
extern crate cmsis_update;
extern crate pack_index as pi;
extern crate pdsc as pack_desc;
extern crate utils;

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
pub mod pack_index;
pub mod pdsc;
pub mod pack;

