extern crate minidom;
extern crate quick_xml;
extern crate futures;
extern crate tokio_core;
extern crate hyper;

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use minidom::Element;
use minidom::Children;
use minidom::error::{Error, ErrorKind};
use quick_xml::reader::Reader;

use std::io::{self, Write};
use futures::{Future, Stream};
use hyper::Client;
use tokio_core::reactor::Core;
use futures::future::Executor;

#[derive(Debug)]
struct Pdsc{
    url: String,
    vendor: String,
    name: String,
    version: String,
    date: Option<String>,
    deprecated: Option<String>,
    replacement: Option<String>,
    size: Option<String>,
}

impl Pdsc {
    fn from_elem(e: &Element) -> Result<Self, Error> {
        let url = (e.attr("url").map(String::from).ok_or(
            Error::from_kind(
                ErrorKind::Msg(String::from("url not found")))))?;
        let vendor = (e.attr("vendor").map(String::from).ok_or(
            Error::from_kind(
                ErrorKind::Msg(String::from("vendor not found")))))?;
        let name = (e.attr("name").map(String::from).ok_or(
            Error::from_kind(
                ErrorKind::Msg(String::from("name not found")))))?;
        let version = (e.attr("version").map(String::from).ok_or(
            Error::from_kind(
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
struct Pidx{
    url: String,
    vendor: String,
    date: Option<String>,
}

impl Pidx {
    fn from_elem(e: &Element) -> Result<Self, Error> {
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
struct Vidx {
    vendor: String,
    url: String,
    timestamp: Option<String>,
    pdscIndex: Option<Vec<Pdsc>>,
    vendorIndex: Option<Vec<Pidx>>,
}

impl Vidx {
    fn from_path(path: &Path) -> Result<Vidx, Error> {
        let mut reader = Reader::from_file(path)?;
        Self::from_reader(& mut reader)
    }

    fn from_str(string: &str) -> Result<Vidx, Error> {
        let mut reader = Reader::from_str(string);
        Self::from_reader(& mut reader)
    }

    fn from_reader<F: std::io::BufRead>(reader: &mut Reader<F>) -> Result<Vidx, Error> {
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

static PIDX_SUFFIX: &'static str = ".pidx";

fn get_all_pdsc(vidx: Vidx) -> Result<Vec<Pdsc>, Error> {
    let mut core = Core::new().unwrap();
    let client = Client::new(&core.handle());
    let mut pdscs = Vec::new();
    if let Some(more) = vidx.pdscIndex {
        pdscs.extend(more);
    }
    if let Some(vidxs) = vidx.vendorIndex {
        for Pidx{url, vendor, ..} in vidxs {
            let mut urlname = String::with_capacity(url.len() + vendor.len() + PIDX_SUFFIX.len());
            urlname += &url;
            urlname += &vendor;
            urlname += &PIDX_SUFFIX;
            match urlname.parse() {
                Ok(uri) => {
                    let work = client.get(uri).and_then(|res|{
                        res.body().concat2().and_then(|body| {
                            if let Ok(next_vidx) = Vidx::from_str(String::from_utf8_lossy(body.as_ref()).into_owned().as_str()){
                                if let Some(more) = next_vidx.pdscIndex {
                                    pdscs.extend(more)
                                }
                            }
                            Ok(())
                        })
                    });
                    core.run(work).unwrap()
                }
                Err(e) => {
                    println!("{}", e)
                }
            }
        }
    }
    Ok(pdscs)
}


fn main() {
    println!("{:#?}", Vidx::from_path(Path::new("keil.vidx")).and_then(get_all_pdsc));
}
