pub(crate) mod parse;
pub(crate) mod prelude;

pub use parse::FromElem;

use std::fmt::Display;
use slog::{Logger, warn, error};

pub trait ResultLogExt<T, E> {
    fn ok_warn(self, log: &Logger) -> Option<T>;
    fn ok_error(self, log: &Logger) -> Option<T>;
}

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