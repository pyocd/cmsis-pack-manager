use std::fmt::Display;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::str::FromStr;

use crate::utils::ResultLogExt;
use roxmltree::{Children, Node};

use anyhow::{format_err, Error};

pub fn attr_map<'a, T>(from: &'a Node, name: &str) -> Result<T, Error>
where
    T: From<&'a str>,
{
    from.attribute(name)
        .map(T::from)
        .ok_or_else(|| format_err!("{} not found in {} element", name, from.tag_name().name()))
}

pub fn attr_parse_hex(from: &Node, name: &str) -> Result<u64, Error> {
    from.attribute(name)
        .ok_or_else(|| format_err!("{} not found in {} element", name, from.tag_name().name()))
        .and_then(|st| {
            if let Some(hex) = st.strip_prefix("0x") {
                u64::from_str_radix(hex, 16).map_err(|e| format_err!("{}", e))
            } else if let Some(oct) = st.strip_prefix('0') {
                u64::from_str_radix(oct, 8).map_err(|e| format_err!("{}", e))
            } else {
                st.parse::<u64>().map_err(|e| format_err!("{}", e))
            }
        })
}

pub fn attr_parse<T, E>(from: &Node, name: &str) -> Result<T, Error>
where
    T: FromStr<Err = E>,
    E: Display,
{
    from.attribute(name)
        .ok_or_else(|| format_err!("{} not found in {} element", name, from.tag_name().name()))
        .and_then(|st| st.parse::<T>().map_err(|e| format_err!("{}", e)))
}

pub fn child_text(from: &Node, name: &str) -> Result<String, Error> {
    for child in from.children() {
        if child.tag_name().name() == name {
            return Ok(child.text().unwrap_or_default().to_string());
        }
    }
    Err(format_err!(
        "child element \"{}\" not found in \"{}\" element",
        name,
        from.tag_name().name()
    ))
}

pub fn assert_root_name(from: &Node, name: &str) -> Result<(), Error> {
    if from.tag_name().name() != name {
        Err(format_err!(
            "tried to parse element \"{}\" from element \"{}\" \"{:?}\"",
            name,
            from.tag_name().name(),
            from,
        ))
    } else {
        Ok(())
    }
}

pub trait FromElem: Sized {
    fn from_elem(e: &Node) -> Result<Self, Error>;

    fn from_string(s: &str) -> Result<Self, Error> {
        let doc = roxmltree::Document::parse(s)?;
        let root = doc.root_element();
        Self::from_elem(&root)
    }

    fn from_reader<T: BufRead>(r: &mut T) -> Result<Self, Error> {
        let mut xml_str = String::new();
        r.read_to_string(&mut xml_str)?;
        Self::from_string(&xml_str)
    }

    fn from_path(p: &Path) -> Result<Self, Error> {
        let f = File::open(p)?;
        let mut r = BufReader::new(f);
        Self::from_reader(&mut r)
    }

    fn vec_from_children(clds: Children) -> Vec<Self> {
        clds.filter(|e| e.is_element())
            .flat_map(move |cld| Self::from_elem(&cld).ok_warn().into_iter())
            .collect()
    }
}
