extern crate minidom;
extern crate quick_xml;

use std::io::BufRead;
use std::path::Path;
use self::minidom::Element;
use self::minidom::Children;
pub use self::minidom::error::{Error, ErrorKind};
use self::quick_xml::reader::Reader;

pub mod network;

#[derive(Debug)]
pub struct Pdsc{
    pub url: String,
    pub vendor: String,
    pub name: String,
    pub version: String,
    pub date: Option<String>,
    pub deprecated: Option<String>,
    pub replacement: Option<String>,
    pub size: Option<String>,
}

impl Pdsc {
    pub fn from_elem(e: &Element) -> Result<Self, Error> {
        let url = (e.attr("url")
                   .map(String::from)
                   .ok_or(Error::from_kind(
                       ErrorKind::Msg(String::from("url not found")))))?;
        let vendor = (e.attr("vendor")
                      .map(String::from)
                      .ok_or(Error::from_kind(
                          ErrorKind::Msg(String::from("vendor not found")))))?;
        let name = (e.attr("name")
                    .map(String::from)
                    .ok_or(Error::from_kind(
                        ErrorKind::Msg(String::from("name not found")))))?;
        let version = (e.attr("version")
                       .map(String::from)
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
    fn vec_from_children(children: Children) -> Option<Vec<Self>>{
        let mut to_ret = Vec::new();
        for e in children {
            if let Ok(pdsc) = Self::from_elem(e) {
                to_ret.push(pdsc);
            }
        }
        Some(to_ret)
    }
}



#[derive(Debug)]
pub struct Pidx{
    pub url: String,
    pub vendor: String,
    pub date: Option<String>,
}

impl Pidx {
    pub fn from_elem(e: &Element) -> Result<Self, Error> {
        let url = (e.attr("url").map(String::from).ok_or(
            Error::from_kind(
                ErrorKind::Msg(String::from("url not found")))))?;
        let vendor = (e.attr("vendor").map(String::from).ok_or(
            Error::from_kind(
                ErrorKind::Msg(String::from("vendor not found")))))?;
        Ok(Self{
            url, vendor,
            date: e.attr("date").map(String::from),
        })
    }
    fn vec_from_children(children: Children) -> Option<Vec<Self>>{
        let mut to_ret = Vec::new();
        for e in children {
            if let Ok(pidx) = Self::from_elem(e) {
                to_ret.push(pidx);
            }
        }
        Some(to_ret)
    }
}

#[derive(Debug)]
pub struct Vidx {
    pub vendor: String,
    pub url: String,
    pub timestamp: Option<String>,
    pub pdscIndex: Option<Vec<Pdsc>>,
    pub vendorIndex: Option<Vec<Pidx>>,
}

static PIDX_SUFFIX: &'static str = ".pidx";

impl Vidx {
    pub fn from_path(path: &Path) -> Result<Vidx, Error> {
        let mut reader = Reader::from_file(path)?;
        Self::from_reader(& mut reader)
    }

    pub fn from_str(string: &str) -> Result<Vidx, Error> {
        let mut reader = Reader::from_str(string);
        Self::from_reader(& mut reader)
    }

    pub fn from_reader<F: BufRead>(reader: &mut Reader<F>) -> Result<Vidx, Error> {
        let root = Element::from_reader(reader)?;
        let vendor = root.get_child("vendor", "http://www.w3.org/2001/XMLSchema-instance")
            .map(Element::text)
            .ok_or(Error::from_kind(
                ErrorKind::Msg(String::from("vendor not found"))))?;
        let url = root.get_child("url", "http://www.w3.org/2001/XMLSchema-instance")
            .map(Element::text)
            .ok_or(Error::from_kind(
                ErrorKind::Msg(String::from("url not found"))))?;
        Ok(Vidx {
            vendor, url,
            timestamp:  root.get_child("timestamp", "http://www.w3.org/2001/XMLSchema-instance").map(Element::text),
            vendorIndex: root.get_child("vindex", "http://www.w3.org/2001/XMLSchema-instance").map(Element::children).and_then(Pidx::vec_from_children),
            pdscIndex: root.get_child("pindex", "http://www.w3.org/2001/XMLSchema-instance").map(Element::children).and_then(Pdsc::vec_from_children),
        })
    }
}
