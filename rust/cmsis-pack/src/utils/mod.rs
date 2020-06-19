pub(crate) mod parse;
pub(crate) mod prelude;

pub use self::parse::FromElem;

use std::fmt::Display;

pub trait ResultLogExt<T, E> {
    fn ok_warn(self) -> Option<T>;
    fn ok_error(self) -> Option<T>;
}

impl<T, E> ResultLogExt<T, E> for Result<T, E>
where
    E: Display,
{
    fn ok_warn(self) -> Option<T> {
        match self {
            Ok(x) => Some(x),
            Err(e) => {
                log::warn!("{}", e);
                None
            }
        }
    }
    fn ok_error(self) -> Option<T> {
        match self {
            Ok(x) => Some(x),
            Err(e) => {
                log::error!("{}", e);
                None
            }
        }
    }
}
