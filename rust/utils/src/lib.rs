extern crate minidom;
extern crate quick_xml;
#[macro_use]
extern crate slog;

pub trait ResultLogExt<T, E> {
    fn ok_warn(self, log: &Logger) -> Option<T>;
    fn ok_error(self, log: &Logger) -> Option<T>;
}

use std::fmt::Display;
use slog::Logger;
impl<T, E> ResultLogExt<T, E> for Result<T, E>
    where
    E: Display,
{
    fn ok_warn(self, log: &Logger) -> Option<T> {
        match self {
            Ok(x) => Some(x),
            Err(e) => {
                warn!(log, "{}", e);
                None
            }
        }
    }
    fn ok_error(self, log: &Logger) -> Option<T> {
        match self {
            Ok(x) => Some(x),
            Err(e) => {
                error!(log, "{}", e);
                None
            }
        }
    }
}

pub mod parse;
