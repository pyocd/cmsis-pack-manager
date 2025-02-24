#![allow(clippy::upper_case_acronyms)]

pub mod pack_index;
pub mod pdsc;
pub mod update;
#[macro_use]
pub mod utils;

extern crate futures;
extern crate log;
extern crate reqwest;
extern crate roxmltree;
extern crate serde;
extern crate serde_json;
extern crate tokio;
