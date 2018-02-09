#![feature(proc_macro, conservative_impl_trait, generators, libc)]

extern crate app_dirs;
extern crate futures_await as futures;
extern crate tokio_core;
extern crate hyper;
extern crate hyper_tls;
extern crate minidom;
extern crate quick_xml;
extern crate utils;
extern crate smallstring;
extern crate clap;
#[macro_use]
extern crate slog;
#[macro_use]
extern crate error_chain;

pub mod network;
pub mod config;
mod parse;
pub use parse::*;
pub use network::*;
