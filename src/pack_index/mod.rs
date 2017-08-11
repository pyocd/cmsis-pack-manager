extern crate smallstring;
extern crate futures;
extern crate tokio_io;
extern crate tokio_core;
extern crate hyper;
extern crate minidom;

use smallstring::SmallString;
use minidom::{Element, Error, ErrorKind};

use ::parse::FromElem;

pub mod network;

#[derive(Debug, Clone)]
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


impl FromElem for PdscRef {
    fn from_elem(e: &Element) -> Result<Self, Error> {
        let url = (e.attr("url")
                   .map(String::from)
                   .ok_or(Error::from_kind(
                       ErrorKind::Msg(String::from("url not found")))))?;
        let vendor = (e.attr("vendor")
                      .map(SmallString::from)
                      .ok_or(Error::from_kind(
                          ErrorKind::Msg(String::from("vendor not found")))))?;
        let name = (e.attr("name")
                    .map(SmallString::from)
                    .ok_or(Error::from_kind(
                        ErrorKind::Msg(String::from("name not found")))))?;
        let version = (e.attr("version")
                       .map(SmallString::from)
                       .ok_or(Error::from_kind(
                           ErrorKind::Msg(String::from("version not found")))))?;
        Ok(Self{
            url, vendor, name, version,
            date: e.attr("date").map(String::from),
            deprecated: e.attr("deprecated").map(String::from),
            replacement: e.attr("replacement").map(String::from),
            size: e.attr("size").map(String::from),
        })
    }
}


impl FromElem for Pidx {
    fn from_elem(e: &Element) -> Result<Self, Error> {
        let url = (e.attr("url").map(String::from).ok_or(
            Error::from_kind(
                ErrorKind::Msg(String::from("url not found")))))?;
        let vendor = (e.attr("vendor").map(SmallString::from).ok_or(
            Error::from_kind(
                ErrorKind::Msg(String::from("vendor not found")))))?;
        Ok(Self{
            url, vendor,
            date: e.attr("date").map(String::from),
        })
    }
}

static DEFAULT_NS: &'static str = "http://www.w3.org/2001/XMLSchema-instance";

impl FromElem for Vidx {
    fn from_elem(root: &Element) -> Result<Self, Error> {
        let vendor = root.get_child("vendor", DEFAULT_NS)
            .map(Element::text)
            .ok_or(Error::from_kind(
                ErrorKind::Msg(String::from("vendor not found"))))?;
        let url = root.get_child("url", DEFAULT_NS)
            .map(Element::text)
            .ok_or(Error::from_kind(
                ErrorKind::Msg(String::from("url not found"))))?;
        Ok(Vidx {
            vendor, url,
            timestamp:  root.get_child("timestamp", DEFAULT_NS)
                .map(Element::text),
            vendor_index: root.get_child("vindex", DEFAULT_NS)
                .map(Element::children)
                .map(Pidx::vec_from_children)
                .unwrap_or(Vec::new()),
            pdsc_index: root.get_child("pindex", DEFAULT_NS)
                .map(Element::children)
                .map(PdscRef::vec_from_children)
                .unwrap_or(Vec::new()),
        })
    }
}
