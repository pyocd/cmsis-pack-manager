extern crate cmsis_pack;
#[macro_use]
extern crate ctor;

#[ctor]
fn cmsis_cffi_init() {
    simplelog::TermLogger::init(
        simplelog::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();
}

macro_rules! with_from_raw {
    (let $boxed:ident = $ptr:ident, $block:block) => {{
        #[allow(unused_unsafe)]
        let $boxed = unsafe { Box::from_raw($ptr) };
        let ret = $block;
        Box::into_raw($boxed);
        ret
    }};
    (let mut $boxed:ident = $ptr:ident, $block:block) => {{
        #[allow(unused_unsafe)]
        let mut $boxed = unsafe { Box::from_raw($ptr) };
        let ret = $block;
        Box::into_raw($boxed);
        ret
    }};
}
#[macro_use]
pub mod utils;

pub mod config;
pub mod pack;
pub mod pack_index;
pub mod pdsc;
