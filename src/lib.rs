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
extern crate slog;
extern crate futures;
extern crate tokio_core;
extern crate hyper;
extern crate hyper_tls;
extern crate minidom;
extern crate quick_xml;
extern crate smallstring;
extern crate xdg;
extern crate clap;

trait ResultLog<T, E> {
    fn ok_warn(self, log: Logger) -> Option<T>;
    fn ok_error(self, log: Logger) -> Option<T>;
}

use std::fmt::Display;
use slog::Logger;
impl<T, E> ResultLog<T, E> for Result<T, E>
where
    E: Display,
{
    fn ok_warn(self, log: Logger) -> Option<T> {
        match self {
            Ok(x) => Some(x),
            Err(e) => {
                warn!(log, "{}", e);
                None
            }
        }
    }
    fn ok_error(self, log: Logger) -> Option<T> {
        match self {
            Ok(x) => Some(x),
            Err(e) => {
                error!(log, "{}", e);
                None
            }
        }
    }
}

pub mod pack_index;
pub mod pdsc;
pub mod parse;
pub mod config;
