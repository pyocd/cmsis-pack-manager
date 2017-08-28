#![feature(conservative_impl_trait)]
#[macro_use]
extern crate custom_derive;
#[macro_use]
extern crate enum_derive;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate futures_error_chain;
#[macro_use]
extern crate log;
extern crate futures;
extern crate tokio_core;
extern crate hyper;
extern crate hyper_tls;
extern crate minidom;
extern crate quick_xml;
extern crate smallstring;
extern crate xdg;
extern crate clap;

pub mod pack_index;
pub mod pdsc;
pub mod parse;
pub mod config;
pub mod logging;
