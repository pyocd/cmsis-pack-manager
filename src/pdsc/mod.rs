use std::error::Error as StdError;
use std::fmt::{self, Display};
use std::path::PathBuf;
use smallstring::SmallString;
use minidom::{Element, Error, ErrorKind};

use ::parse::{attr_map, attr_parse, FromElem};

custom_derive!{
    #[derive(Debug, PartialEq, Eq, EnumFromStr)]
    pub enum FileCategory{
        doc,
        header,
        include,
        library,
        object,
        source,
        sourceC,
        sourceCpp,
        sourceAsm,
        linkerScript,
        utility,
        image,
        other,
    }
}

custom_derive!{
    #[derive(Debug, PartialEq, Eq, EnumFromStr)]
    pub enum FileAttribute{
        config, template
    }
}

pub struct FileRef{
    path:      PathBuf,
    category:  FileCategory,
    attr:      Option<FileAttribute>,
    condition: Option<String>,
    select:    Option<String>,
    src:       Option<String>,
    version:   Option<String>,
}


impl FromElem for FileRef {
    fn from_elem(e: &Element) -> Result<Self, Error>{
        Ok(Self{
            path:      attr_map(e, "path", "file")?,
            category:  attr_parse(e, "category", "file")?,
            attr:      attr_parse(e, "attr", "file").ok(),
            condition: attr_map(e, "condition", "file").ok(),
            select:    attr_map(e, "select", "file").ok(),
            src:       attr_map(e, "src", "file").ok(),
            version:   attr_map(e, "version", "file").ok(),
        })
    }
}
