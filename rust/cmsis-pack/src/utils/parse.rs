use std::fmt::Display;
use std::io::BufRead;
use std::path::Path;
use std::str::FromStr;

use crate::utils::ResultLogExt;
use minidom::quick_xml::Reader;
use minidom::{Children, Element};

use anyhow::{format_err, Error};

pub fn attr_map<'a, T>(from: &'a Element, name: &str, elemname: &'static str) -> Result<T, Error>
where
    T: From<&'a str>,
{
    from.attr(name)
        .map(T::from)
        .ok_or_else(|| format_err!("{} not found in {} element", name, elemname))
}

pub fn attr_parse_hex<'a>(
    from: &'a Element,
    name: &str,
    elemname: &'static str,
) -> Result<u64, Error> {
    from.attr(name)
        .ok_or_else(|| format_err!("{} not found in {} element", name, elemname))
        .and_then(|st| {
            if st.starts_with("0x") {
                u64::from_str_radix(&st[2..], 16).map_err(|e| format_err!("{}", e))
            } else if st.starts_with('0') {
                u64::from_str_radix(&st[1..], 8).map_err(|e| format_err!("{}", e))
            } else {
                u64::from_str_radix(st, 10).map_err(|e| format_err!("{}", e))
            }
        })
}

pub fn attr_parse<'a, T, E>(
    from: &'a Element,
    name: &str,
    elemname: &'static str,
) -> Result<T, Error>
where
    T: FromStr<Err = E>,
    E: Display,
{
    from.attr(name)
        .ok_or_else(|| format_err!("{} not found in {} element", name, elemname))
        .and_then(|st| st.parse::<T>().map_err(|e| format_err!("{}", e)))
}

pub fn child_text<'a>(
    from: &'a Element,
    name: &str,
    elemname: &'static str,
) -> Result<String, Error> {
    match get_child_no_ns(from, name) {
        Some(child) => Ok(child.text()),
        None => Err(format_err!(
            "child element \"{}\" not found in \"{}\" element",
            name,
            elemname
        )),
    }
}

pub fn get_child_no_ns<'a>(from: &'a Element, name: &str) -> Option<&'a Element> {
    for child in from.children() {
        if child.name() == name {
            return Some(child);
        }
    }
    None
}

pub fn assert_root_name(from: &Element, name: &str) -> Result<(), Error> {
    if from.name() != name {
        Err(format_err!(
            "tried to parse element \"{}\" from element \"{}\"",
            name,
            from.name()
        ))
    } else {
        Ok(())
    }
}

pub trait FromElem: Sized {
    fn from_elem(e: &Element) -> Result<Self, Error>;

    fn from_reader<T: BufRead>(r: &mut Reader<T>) -> Result<Self, Error> {
        let mut root = Element::from_reader(r)?;
        root.set_attr::<&str, Option<String>>("xmlns:xs", None);
        Self::from_elem(&root)
    }
    fn from_string(s: &str) -> Result<Self, Error> {
        let mut r = Reader::from_str(s);
        Self::from_reader(&mut r)
    }
    fn from_path(p: &Path) -> Result<Self, Error> {
        let mut r = Reader::from_file(p)?;
        Self::from_reader(&mut r)
    }
    fn vec_from_children(clds: Children) -> Vec<Self> {
        clds.flat_map(move |cld| Self::from_elem(cld).ok_warn().into_iter())
            .collect()
    }
}
