extern crate smallstring;

use self::smallstring::SmallString;

pub mod network;
pub mod parse;

#[derive(Debug)]
pub struct PdscRef{
    pub url: String,
    pub vendor: SmallString,
    pub name: SmallString,
    pub version: SmallString,
    pub date: Option<String>,
    pub deprecated: Option<String>,
    pub replacement: Option<String>,
    pub size: Option<String>,
}

#[derive(Debug)]
pub struct Pidx{
    pub url: String,
    pub vendor: SmallString,
    pub date: Option<String>,
}


#[derive(Debug)]
pub struct Vidx {
    pub vendor: String,
    pub url: String,
    pub timestamp: Option<String>,
    pub pdsc_index: Vec<PdscRef>,
    pub vendor_index: Vec<Pidx>,
}

