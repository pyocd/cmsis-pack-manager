use std::path::{Path, PathBuf};
use minidom::{Element, Error};
use clap::{App, Arg, ArgMatches, SubCommand};

use ::parse::{attr_map, attr_parse, child_text, FromElem, DEFAULT_NS};
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

#[derive(Debug)]
struct Component{
    vendor:        Option<String>,
    class:         Option<String>,
    group:         Option<String>,
    sub_group:     Option<String>,
    variant:       Option<String>,
    version:       Option<String>,
    api_version:   Option<String>,
    condition:     Option<String>,
    max_instances: Option<u8>,
    is_default:    bool,
    deprecated:    bool,
    description:   String,
    rte_addition:  String,
    files:         Vec<FileRef>,
}

impl FromElem for Component{
    fn from_elem(e: &Element) -> Result<Self, Error> {
        let files = e.get_child("files", DEFAULT_NS)
            .map(|child| FileRef::vec_from_children(child.children()))
            .unwrap_or_default();
        Ok(Self{
            vendor:        attr_map(e, "Cvendor", "component").ok(),
            class:         attr_map(e, "Cclass", "component").ok(),
            group:         attr_map(e, "Cgroup", "component").ok(),
            sub_group:     attr_map(e, "Csub", "component").ok(),
            version:       attr_map(e, "Cversion", "component").ok(),
            variant:       attr_map(e, "Cvariant", "component").ok(),
            api_version:   attr_map(e, "Capiversion", "component").ok(),
            condition:     attr_map(e, "condition", "component").ok(),
            max_instances: attr_parse(e, "maxInstances", "component").ok(),
            is_default:    attr_parse(e, "isDefaultVariant", "component")
                .unwrap_or(true),
            description:   child_text(e, "description", "component")?,
            deprecated:    child_text(e, "deprecated", "component")
                .map(|s| s.parse().unwrap_or(false))
                .unwrap_or(false),
            rte_addition:  child_text(e, "RTE_components_h", "component").unwrap_or_default(),
            files
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
    println!("{:#?}", Component::from_path(Path::new(filename)));
    Ok(())
}
