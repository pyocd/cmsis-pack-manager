use std::path::Path;
use std::io::BufRead;

use minidom::{Element, Children, Error};
use quick_xml::reader::Reader;

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
        clds.flat_map(|cld|{
            Self::from_elem(cld).into_iter()
        }).collect()
    }
}
