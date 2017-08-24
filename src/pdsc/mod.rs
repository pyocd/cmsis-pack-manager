use std::path::{Path, PathBuf};
use minidom::{Element, Error};
use clap::{App, Arg, ArgMatches, SubCommand};

use ::parse::{attr_map, attr_parse, FromElem};
use ::config::Config;
use ::pack_index::network::Error as NetError;

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

#[derive(Debug)]
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
            path:      attr_map(e, "name", "file")?,
            category:  attr_parse(e, "category", "file")?,
            attr:      attr_parse(e, "attr", "file").ok(),
            condition: attr_map(e, "condition", "file").ok(),
            select:    attr_map(e, "select", "file").ok(),
            src:       attr_map(e, "src", "file").ok(),
            version:   attr_map(e, "version", "file").ok(),
        })
    }
}

pub fn check_args<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("check")
        .about("Check a project or pack for correct usage of the CMSIS standard")
        .version("0.1.0")
        .arg(Arg::with_name("INPUT")
             .help("Input file to check")
             .required(true)
             .index(1))
}

pub fn check_command<'a> (_: &Config, args: &ArgMatches<'a>) -> Result<(), NetError> {
    let filename = args.value_of("INPUT").unwrap();
    println!("{:#?}", FileRef::from_path(Path::new(filename)));
    Ok(())
}
