use std::str::FromStr;
use std::fmt::Display;
use std::path::Path;
use std::io::BufRead;

use minidom::{Element, Children, Error};
use quick_xml::reader::Reader;
use slog::Logger;
use super::ResultLogExt;

#[macro_export]
macro_rules! err_msg {
    ($($arg:tt)*) => {
        {
            use minidom::ErrorKind;
            Error::from_kind(ErrorKind::Msg(format!($($arg)*)))
        }
    };
}

pub fn attr_map<'a, T>(from: &'a Element, name: &str, elemname: &'static str) -> Result<T, Error>
where
    T: From<&'a str>,
{
    from.attr(name).map(T::from).ok_or_else(||
        err_msg!("{} not found in {} element", name, elemname))
}

pub fn attr_parse_hex<'a>(
    from: &'a Element,
    name: &str,
    elemname: &'static str,
) -> Result<u64, Error>
{
    from.attr(name)
        .ok_or_else(|| err_msg!("{} not found in {} element", name, elemname))
        .and_then(|st| {
            if st.starts_with("0x") {
                u64::from_str_radix(&st[2..], 16).map_err(|e| err_msg!("{}", e))
            } else if st.starts_with('0') {
                u64::from_str_radix(&st[1..], 8).map_err(|e| err_msg!("{}", e))
            } else {
                u64::from_str_radix(st, 10).map_err(|e| err_msg!("{}", e))
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
        .ok_or_else(|| err_msg!("{} not found in {} element", name, elemname))
        .and_then(|st| {
            st.parse::<T>().map_err(|e| err_msg!("{}", e))
        })
}

pub fn child_text<'a>(
    from: &'a Element,
    name: &str,
    elemname: &'static str,
) -> Result<String, Error> {
    match get_child_no_ns(from, name) {
        Some(child) => {Ok(child.text())}
        None => {Err(err_msg!(
            "child element \"{}\" not found in \"{}\" element",
            name,
            elemname))}

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
        Err(err_msg!(
            "tried to parse element \"{}\" from element \"{}\"",
            name,
            from.name()
        ))
    } else {
        Ok(())
    }
}


pub trait FromElem: Sized {
    fn from_elem(e: &Element, l: &Logger) -> Result<Self, Error>;

    fn from_reader<T: BufRead>(r: &mut Reader<T>, l: &Logger) -> Result<Self, Error> {
        let mut root = Element::from_reader(r)?;
        root.set_attr::<&str, Option<String>>("xmlns:xs", None);
        Self::from_elem(&root, l)
    }
    fn from_string(s: &str, l: &Logger) -> Result<Self, Error> {
        let mut r = Reader::from_str(s);
        Self::from_reader(&mut r, l)
    }
    fn from_path(p: &Path, l: &Logger) -> Result<Self, Error> {
        let mut r = Reader::from_file(p)?;
        Self::from_reader(&mut r, l)
    }
    fn vec_from_children(clds: Children, l: &Logger) -> Vec<Self> {
        clds.flat_map(move |cld| {
            Self::from_elem(cld, l)
                .ok_warn(l)
                .into_iter()
        }).collect()
    }
}
