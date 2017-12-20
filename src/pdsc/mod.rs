use std::path::{Path, PathBuf};
use minidom::{Element, Error, ErrorKind};
use clap::{App, Arg, ArgMatches, SubCommand};
use slog::Logger;

use parse::{attr_map, attr_parse, child_text, assert_root_name, FromElem, DEFAULT_NS};
use config::Config;
use pack_index::network::Error as NetError;
use ResultLogExt;

custom_derive!{
    #[derive(Debug, PartialEq, Eq, EnumFromStr, Clone)]
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
    #[derive(Debug, PartialEq, Eq, EnumFromStr, Clone)]
    pub enum FileAttribute{
        config, template
    }
}

#[derive(Debug, Clone)]
pub struct FileRef {
    path: PathBuf,
    category: FileCategory,
    attr: Option<FileAttribute>,
    condition: Option<String>,
    select: Option<String>,
    src: Option<String>,
    version: Option<String>,
}

impl FromElem for FileRef {
    fn from_elem(e: &Element, l: &Logger) -> Result<Self, Error> {
        assert_root_name(e, "file")?;
        Ok(Self {
            path: attr_map(e, "name", "file")?,
            category: attr_parse(e, "category", "file")?,
            attr: attr_parse(e, "attr", "file").ok(),
            condition: attr_map(e, "condition", "file").ok(),
            select: attr_map(e, "select", "file").ok(),
            src: attr_map(e, "src", "file").ok(),
            version: attr_map(e, "version", "file").ok(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ComponentBuilder{
    vendor: Option<String>,
    class: Option<String>,
    group: Option<String>,
    sub_group: Option<String>,
    variant: Option<String>,
    version: Option<String>,
    api_version: Option<String>,
    condition: Option<String>,
    max_instances: Option<u8>,
    is_default: bool,
    deprecated: bool,
    description: String,
    rte_addition: String,
    files: Vec<FileRef>,
}

impl FromElem for ComponentBuilder{
    fn from_elem(e: &Element, l: &Logger) -> Result<Self, Error> {
        assert_root_name(e, "component")?;
        let mut l = l.new(o!("in" => "Component"));
        let vendor: Option<String> = attr_map(e, "Cvendor", "component").ok();
        if let Some(v) = vendor.clone() {
            l = l.new(o!("Vendor" => v));
        }
        let class: Option<String> = attr_map(e, "Cclass", "component").ok();
        if let Some(c) = class.clone() {
            l = l.new(o!("Class" => c));
        }
        let group: Option<String> = attr_map(e, "Cgroup", "component").ok();
        if let Some(g) = group.clone() {
            l = l.new(o!("Group" => g));
        }
        let sub_group: Option<String> = attr_map(e, "Csub", "component").ok();
        if let Some(s) = vendor.clone() {
            l = l.new(o!("SubGroup" => s));
        }
        let files = e.get_child("files", DEFAULT_NS)
            .map(move |child| {
                FileRef::vec_from_children(child.children(), &l)
            })
            .unwrap_or_default();
        Ok(Self {
            vendor,
            class,
            group,
            sub_group,
            version: attr_map(e, "Cversion", "component").ok(),
            variant: attr_map(e, "Cvariant", "component").ok(),
            api_version: attr_map(e, "Capiversion", "component").ok(),
            condition: attr_map(e, "condition", "component").ok(),
            max_instances: attr_parse(e, "maxInstances", "component").ok(),
            is_default: attr_parse(e, "isDefaultVariant", "component").unwrap_or(true),
            description: child_text(e, "description", "component")?,
            deprecated: child_text(e, "deprecated", "component")
                .map(|s| s.parse().unwrap_or(false))
                .unwrap_or(false),
            rte_addition: child_text(e, "RTE_components_h", "component").unwrap_or_default(),
            files,
        })
    }
}

#[derive(Debug)]
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
    pub fn into_components(self, l: &Logger) -> Vec<ComponentBuilder> {
        let class = self.class;
        let version = self.version;
        let vendor = self.vendor;
        if self.components.is_empty() {
            let mut l = l.new(o!("in" => "Bundle",
                                 "Class" => class.clone()));
            if let Some(v) = vendor.clone() {
                l = l.new(o!("Vendor" => v));
            }
            warn!(l, "Bundle should not be empty")
        }
        self.components
            .into_iter()
            .map(|comp| {
                ComponentBuilder {
                    class: comp.class.or_else(|| Some(class.clone())),
                    version: comp.version.or_else(|| Some(version.clone())),
                    vendor: comp.vendor.or_else(|| vendor.clone()),
                    ..comp
                }
            })
            .collect()
    }
}

impl FromElem for Bundle {
    fn from_elem(e: &Element, l: &Logger) -> Result<Self, Error> {
        assert_root_name(e, "bundle")?;
        let name: String = attr_map(e, "Cbundle", "bundle")?;
        let class: String = attr_map(e, "Cclass", "bundle")?;
        let version: String = attr_map(e, "Cversion", "bundle")?;
        let l = l.new(o!("Bundle" => name.clone(),
                         "Class" => class.clone(),
                         "Version" => version.clone()));
        let components = e.children()
            .filter_map(move |chld| if chld.name() == "component" {
                ComponentBuilder::from_elem(chld, &l).ok()
            } else {
                None
            })
            .collect();
        Ok(Self {
            name,
            class,
            version,
            vendor: attr_map(e, "Cvendor", "bundle").ok(),
            description: child_text(e, "description", "bundle")?,
            doc: child_text(e, "doc", "bundle")?,
            components,
        })
    }
}

fn child_to_component_iter(
    e: &Element,
    l: &Logger,
) -> Result<Box<Iterator<Item = ComponentBuilder>>, Error> {
    match e.name() {
        "bundle" => {
            let bundle = Bundle::from_elem(e, l)?;
            Ok(Box::new(bundle.into_components(l).into_iter()))
        }
        "component" => {
            let component = ComponentBuilder::from_elem(e, l)?;
            Ok(Box::new(Some(component).into_iter()))
        }
        _ => {
            Err(Error::from_kind(ErrorKind::Msg(String::from(format!(
                "element of name {} is not allowed as a descendant of components",
                e.name()
            )))))
        }
    }
}

type ComponentBuilders = Vec<ComponentBuilder>;

impl FromElem for ComponentBuilders {
    fn from_elem(e: &Element, l: &Logger) -> Result<Self, Error> {
        assert_root_name(e, "components")?;
        Ok(
            e.children()
                .flat_map(move |c| match child_to_component_iter(c, l) {
                    Ok(iter) => iter,
                    Err(e) => {
                        error!(l, "when trying to parse component: {}", e);
                        Box::new(None.into_iter())
                    }
                })
                .collect(),
        )
    }
}

struct Release {
    version: String,
    text: String,
}

impl FromElem for Release {
    fn from_elem(e: &Element, l: &Logger) -> Result<Self, Error> {
        assert_root_name(e, "release")?;
        Ok(Self{
            version: attr_map(e, "version", "release")?,
            text: e.text(),
        })
    }
}

type Releases = Vec<Release>;

impl FromElem for Releases {
    fn from_elem(e: &Element, l: &Logger) -> Result<Self, Error> {
        assert_root_name(e, "releases")?;
        Ok(
            e.children()
                .flat_map(|c| Release::from_elem(c, l).ok_warn(l))
                .collect()
        )
    }
}

struct Package {
    name: String,
    description: String,
    vendor: String,
    url: String,
    license: Option<String>,
    pub components: ComponentBuilders,
    pub releases: Releases,
}

impl FromElem for Package {
    fn from_elem(e: &Element, l: &Logger) -> Result<Self, Error> {
        assert_root_name(e, "package")?;
        let name: String = child_text(e, "name", "package")?;
        let description: String = child_text(e, "description", "package")?;
        let vendor: String = child_text(e, "vendor", "package")?;
        let url: String = child_text(e, "url", "package")?;
        let l = l.new(o!("Vendor" => vendor.clone(),
                         "Package" => name.clone()
        ));
        let components = e.get_child("components", DEFAULT_NS)
            .and_then(|c| ComponentBuilders::from_elem(c, &l).ok_warn(&l))
            .unwrap_or_default();
        let releases = e.get_child("releases", DEFAULT_NS)
            .and_then(|c| Releases::from_elem(c, &l).ok_warn(&l))
            .unwrap_or_default();
        Ok(Self {
            name,
            description,
            vendor,
            url,
            components,
            license: child_text(e, "license", "package").ok_warn(&l),
            releases,
        })
    }
}


#[derive(Debug)]
pub struct Component{
    vendor: String,
    class: String,
    group: String,
    sub_group: Option<String>,
    variant: Option<String>,
    version: String,
    api_version: Option<String>,
    condition: Option<String>,
    max_instances: Option<u8>,
    is_default: bool,
    deprecated: bool,
    description: String,
    rte_addition: String,
    files: Vec<FileRef>,
}

type Components = Vec<Component>;

impl Package {
    fn make_components(&self) -> Components {
        self.components.clone().into_iter().map(|comp| {
            Component{
                vendor: comp.vendor.unwrap_or_else(|| self.vendor.clone()),
                class: comp.class.unwrap(),
                group: comp.group.unwrap(),
                sub_group: comp.sub_group,
                variant: comp.variant,
                version: comp.version.unwrap_or_else(|| self.releases[0].version.clone()),
                api_version: comp.api_version,
                condition: comp.condition,
                max_instances: comp.max_instances,
                is_default: comp.is_default,
                deprecated: comp.deprecated,
                description: comp.description,
                rte_addition: comp.rte_addition,
                files: comp.files,
            }
        }).collect()
    }
}

pub fn check_args<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("check")
        .about(
            "Check a project or pack for correct usage of the CMSIS standard",
        )
        .version("0.1.0")
        .arg(
            Arg::with_name("INPUT")
                .help("Input file to check")
                .required(true)
                .index(1),
        )
}

pub fn check_command<'a>(_: &Config, args: &ArgMatches<'a>, l: &Logger) -> Result<(), NetError> {
    let filename = args.value_of("INPUT").unwrap();
    match Package::from_path(Path::new(filename.clone()), &l) {
        Ok(c) => {
            info!(l, "Parsing succedded");
            println!("Software Components:");
            for &Component{ref vendor, ref class, ref group, ref sub_group, ref condition, ref files, ..} in c.make_components().iter() {
                println!("  {}::{}::{}::{:?} condition {:?}", vendor, class, group, sub_group, condition);
                for &FileRef{ref path, ref category, ref condition, ..} in files.iter() {
                    println!("    {:?}  {:?} condition {:?}", category, path, condition);
                }
            }
        }
        Err(e) => {
            error!(l, "parsing {}: {}", filename, e);
        }
    }
    debug!(l, "exiting");
    Ok(())
}
