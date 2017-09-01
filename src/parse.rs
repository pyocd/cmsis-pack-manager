use std::str::FromStr;
use std::fmt::Display;
use std::path::Path;
use std::io::BufRead;

use minidom::{Element, Children, Error, ErrorKind};
use quick_xml::reader::Reader;

pub static DEFAULT_NS: &'static str = "http://www.w3.org/2001/XMLSchema-instance";

pub fn attr_map<'a, T>(from: &'a Element, name: &str, elemname: &'static str) -> Result<T, Error>
where
    T: From<&'a str>,
{
    from.attr(name).map(T::from).ok_or_else(|| {
        Error::from_kind(ErrorKind::Msg(String::from(
            format!("{} not found in {} element", name, elemname),
        )))
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
        .ok_or_else(|| {
            Error::from_kind(ErrorKind::Msg(String::from(
                format!("{} not found in {} element", name, elemname),
            )))
        })
        .and_then(|st| {
            st.parse::<T>().map_err(|e| {
                Error::from_kind(ErrorKind::Msg(String::from(format!("{}", e))))
            })
        })
}

pub fn child_text<'a>(
    from: &'a Element,
    name: &str,
    elemname: &'static str,
) -> Result<String, Error> {
    from.get_child(name, DEFAULT_NS)
        .map(Element::text)
        .ok_or_else(|| {
            Error::from_kind(ErrorKind::Msg(String::from(format!(
                "child element \"{}\" not found in \"{}\" element",
                name,
                elemname
            ))))
        })
}

pub fn assert_root_name(from: &Element, name: &str) -> Result<(), Error> {
    if from.name() != name {
        Err(Error::from_kind(ErrorKind::Msg(String::from(format!(
            "tried to parse element \"{}\" from element \"{}\"",
            name, from.name()
        )))))
    } else {
        Ok(())
    }
}


pub trait FromElem: Sized {
    fn from_elem(e: &Element) -> Result<Self, Error>;

    fn from_reader<T: BufRead>(r: &mut Reader<T>) -> Result<Self, Error> {
        let root = Element::from_reader(r)?;
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
        clds.flat_map(|cld| Self::from_elem(cld).into_iter())
            .collect()
    }
}
