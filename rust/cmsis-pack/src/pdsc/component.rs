use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{format_err, Error};
use minidom::Element;
use serde::Serialize;

use crate::utils::prelude::*;

#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
pub enum FileCategory {
    Doc,
    Header,
    Include,
    Library,
    Object,
    Source,
    SourceC,
    SourceCpp,
    SourceAsm,
    LinkerScript,
    Utility,
    Image,
    PreIncludeGlobal,
    PreIncludeLocal,
    Other,
}

impl FromStr for FileCategory {
    type Err = Error;
    fn from_str(from: &str) -> Result<Self, Error> {
        match from {
            "doc" => Ok(FileCategory::Doc),
            "header" => Ok(FileCategory::Header),
            "include" => Ok(FileCategory::Include),
            "library" => Ok(FileCategory::Library),
            "object" => Ok(FileCategory::Object),
            "source" => Ok(FileCategory::Source),
            "sourceC" => Ok(FileCategory::SourceC),
            "sourceCpp" => Ok(FileCategory::SourceCpp),
            "sourceAsm" => Ok(FileCategory::SourceAsm),
            "linkerScript" => Ok(FileCategory::LinkerScript),
            "utility" => Ok(FileCategory::Utility),
            "image" => Ok(FileCategory::Image),
            "preIncludeGlobal" => Ok(FileCategory::PreIncludeGlobal),
            "preIncludeLocal" => Ok(FileCategory::PreIncludeLocal),
            "other" => Ok(FileCategory::Other),
            unknown => Err(format_err!("Unknown file category {}", unknown)),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
pub enum FileAttribute {
    Config,
    Template,
}

impl FromStr for FileAttribute {
    type Err = Error;
    fn from_str(from: &str) -> Result<Self, Error> {
        match from {
            "config" => Ok(FileAttribute::Config),
            "template" => Ok(FileAttribute::Template),
            unknown => Err(format_err!("Unknown file attribute {}", unknown)),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FileRef {
    pub path: PathBuf,
    category: FileCategory,
    attr: Option<FileAttribute>,
    pub condition: Option<String>,
    select: Option<String>,
    src: Option<String>,
    version: Option<String>,
}

impl FromElem for FileRef {
    fn from_elem(e: &Element) -> Result<Self, Error> {
        assert_root_name(e, "file")?;
        Ok(Self {
            path: attr_map(e, "name")?,
            category: attr_parse(e, "category")?,
            attr: attr_parse(e, "attr").ok(),
            condition: attr_map(e, "condition").ok(),
            select: attr_map(e, "select").ok(),
            src: attr_map(e, "src").ok(),
            version: attr_map(e, "version").ok(),
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ComponentBuilder {
    pub vendor: Option<String>,
    pub class: Option<String>,
    pub group: Option<String>,
    pub sub_group: Option<String>,
    pub variant: Option<String>,
    pub version: Option<String>,
    pub api_version: Option<String>,
    pub condition: Option<String>,
    pub max_instances: Option<u8>,
    pub is_default: bool,
    pub deprecated: bool,
    pub description: String,
    pub rte_addition: String,
    pub files: Vec<FileRef>,
}

impl FromElem for ComponentBuilder {
    fn from_elem(e: &Element) -> Result<Self, Error> {
        assert_root_name(e, "component")?;
        let vendor: Option<String> = attr_map(e, "Cvendor").ok();
        let class: Option<String> = attr_map(e, "Cclass").ok();
        let group: Option<String> = attr_map(e, "Cgroup").ok();
        let sub_group: Option<String> = attr_map(e, "Csub").ok();
        let vendor_string = vendor.clone().unwrap_or_else(|| "Vendor".into());
        let class_string = class.clone().unwrap_or_else(|| "Class".into());
        let group_string = group.clone().unwrap_or_else(|| "Group".into());
        let sub_group_string = sub_group.clone().unwrap_or_else(|| "SubGroup".into());
        let files = get_child_no_ns(e, "files")
            .map(move |child| {
                log::debug!(
                    "Working on {}::{}::{}::{}",
                    vendor_string,
                    class_string,
                    group_string,
                    sub_group_string,
                );
                FileRef::vec_from_children(child.children())
            })
            .unwrap_or_default();
        Ok(Self {
            vendor,
            class,
            group,
            sub_group,
            version: attr_map(e, "Cversion").ok(),
            variant: attr_map(e, "Cvariant").ok(),
            api_version: attr_map(e, "Capiversion").ok(),
            condition: attr_map(e, "condition").ok(),
            max_instances: attr_parse(e, "maxInstances").ok(),
            is_default: attr_parse(e, "isDefaultVariant").unwrap_or(true),
            description: child_text(e, "description")?,
            deprecated: child_text(e, "deprecated")
                .map(|s| s.parse().unwrap_or(false))
                .unwrap_or(false),
            rte_addition: child_text(e, "RTE_components_h").unwrap_or_default(),
            files,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct Bundle {
    name: String,
    class: String,
    version: String,
    vendor: Option<String>,
    description: String,
    doc: String,
    components: Vec<ComponentBuilder>,
}

impl Bundle {
    pub fn into_components(self) -> Vec<ComponentBuilder> {
        let class = self.class;
        let version = self.version;
        let vendor = self.vendor;
        if self.components.is_empty() {
            log::warn!("Bundle should not be empty")
        }
        self.components
            .into_iter()
            .map(|comp| ComponentBuilder {
                class: comp.class.or_else(|| Some(class.clone())),
                version: comp.version.or_else(|| Some(version.clone())),
                vendor: comp.vendor.or_else(|| vendor.clone()),
                ..comp
            })
            .collect()
    }
}

impl FromElem for Bundle {
    fn from_elem(e: &Element) -> Result<Self, Error> {
        assert_root_name(e, "bundle")?;
        let name: String = attr_map(e, "Cbundle")?;
        let class: String = attr_map(e, "Cclass")?;
        let version: String = attr_map(e, "Cversion")?;
        // let l = l.new(o!("Bundle" => name.clone(),
        //                  "Class" => class.clone(),
        //                  "Version" => version.clone()));
        let components = e
            .children()
            .filter_map(move |chld| {
                if chld.name() == "component" {
                    ComponentBuilder::from_elem(chld).ok()
                } else {
                    None
                }
            })
            .collect();
        Ok(Self {
            name,
            class,
            version,
            vendor: attr_map(e, "Cvendor").ok(),
            description: child_text(e, "description")?,
            doc: child_text(e, "doc")?,
            components,
        })
    }
}

fn child_to_component_iter(
    e: &Element,
) -> Result<Box<dyn Iterator<Item = ComponentBuilder>>, Error> {
    match e.name() {
        "bundle" => {
            let bundle = Bundle::from_elem(e)?;
            Ok(Box::new(bundle.into_components().into_iter()))
        }
        "component" => {
            let component = ComponentBuilder::from_elem(e)?;
            Ok(Box::new(Some(component).into_iter()))
        }
        _ => Err(format_err!(
            "element of name {} is not allowed as a descendant of components",
            e.name()
        )),
    }
}

#[derive(Default)]
pub struct ComponentBuilders(pub(crate) Vec<ComponentBuilder>);

impl FromElem for ComponentBuilders {
    fn from_elem(e: &Element) -> Result<Self, Error> {
        assert_root_name(e, "components")?;
        Ok(ComponentBuilders(
            e.children()
                .flat_map(move |c| match child_to_component_iter(c) {
                    Ok(iter) => iter,
                    Err(e) => {
                        log::error!("when trying to parse component: {}", e);
                        Box::new(None.into_iter())
                    }
                })
                .collect(),
        ))
    }
}
