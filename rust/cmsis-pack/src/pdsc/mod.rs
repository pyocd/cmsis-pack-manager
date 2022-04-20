use minidom::Element;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::fs::OpenOptions;
use std::io::Read;
use std::path::Path;

use crate::utils::prelude::*;
use anyhow::{format_err, Error};

mod component;
mod condition;
mod device;
pub use component::{ComponentBuilders, FileRef};
pub use condition::{Condition, Conditions};
pub use device::{Algorithm, Core, Device, Devices, Memories, Processor};

pub struct Release {
    pub version: String,
    pub text: String,
}

impl FromElem for Release {
    fn from_elem(e: &Element) -> Result<Self, Error> {
        assert_root_name(e, "release")?;
        Ok(Self {
            version: attr_map(e, "version", "release")?,
            text: e.text(),
        })
    }
}

#[derive(Default)]
pub struct Releases(Vec<Release>);

impl Releases {
    pub fn latest_release(&self) -> &Release {
        &self.0[0]
    }
}

impl FromElem for Releases {
    fn from_elem(e: &Element) -> Result<Self, Error> {
        assert_root_name(e, "releases")?;
        let to_ret: Vec<_> = e
            .children()
            .flat_map(|c| Release::from_elem(c).ok_warn())
            .collect();
        if to_ret.is_empty() {
            Err(format_err!("There must be at least one release!"))
        } else {
            Ok(Releases(to_ret))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DumpDevice<'a> {
    name: &'a str,
    memories: Cow<'a, Memories>,
    algorithms: Cow<'a, Vec<Algorithm>>,
    processors: Cow<'a, Vec<Processor>>,
    from_pack: FromPack<'a>,
    vendor: Option<&'a str>,
    family: &'a str,
    sub_family: Option<&'a str>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct FromPack<'a> {
    vendor: &'a str,
    pack: &'a str,
    version: &'a str,
    url: &'a str,
}

impl<'a> FromPack<'a> {
    fn new(vendor: &'a str, pack: &'a str, version: &'a str, url: &'a str) -> Self {
        Self {
            vendor,
            pack,
            version,
            url,
        }
    }
}

impl<'a> DumpDevice<'a> {
    fn from_device(dev: &'a Device, from_pack: FromPack<'a>) -> Self {
        Self {
            name: &dev.name,
            memories: Cow::Borrowed(&dev.memories),
            algorithms: Cow::Borrowed(&dev.algorithms),
            processors: Cow::Borrowed(&dev.processors),
            from_pack,
            vendor: dev.vendor.as_deref(),
            family: &dev.family,
            sub_family: dev.sub_family.as_deref(),
        }
    }
}

pub struct Package {
    pub name: String,
    pub description: String,
    pub vendor: String,
    pub url: String,
    pub license: Option<String>,
    components: ComponentBuilders,
    pub releases: Releases,
    pub conditions: Conditions,
    pub devices: Devices,
    pub boards: Vec<Board>,
}

impl FromElem for Package {
    fn from_elem(e: &Element) -> Result<Self, Error> {
        assert_root_name(e, "package")?;
        let name: String = child_text(e, "name", "package")?;
        let description: String = child_text(e, "description", "package")?;
        let vendor: String = child_text(e, "vendor", "package")?;
        let url: String = child_text(e, "url", "package")?;
        log::debug!("Working on {}::{}", vendor, name,);
        let components = get_child_no_ns(e, "components")
            .and_then(|c| ComponentBuilders::from_elem(c).ok_warn())
            .unwrap_or_default();
        let releases = get_child_no_ns(e, "releases")
            .and_then(|c| Releases::from_elem(c).ok_warn())
            .unwrap_or_default();
        let conditions = get_child_no_ns(e, "conditions")
            .and_then(|c| Conditions::from_elem(c).ok_warn())
            .unwrap_or_default();
        let devices = get_child_no_ns(e, "devices")
            .and_then(|c| Devices::from_elem(c).ok_warn())
            .unwrap_or_default();
        let boards = get_child_no_ns(e, "boards")
            .map(|c| Board::vec_from_children(c.children()))
            .unwrap_or_default();
        Ok(Self {
            name,
            description,
            vendor,
            url,
            components,
            license: child_text(e, "license", "package").ok(),
            releases,
            conditions,
            devices,
            boards,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Board {
    name: String,
    mounted_devices: Vec<String>,
}

impl FromElem for Board {
    fn from_elem(e: &Element) -> Result<Self, Error> {
        Ok(Self {
            name: attr_map(e, "name", "board")?,
            mounted_devices: e
                .children()
                .flat_map(|c| match c.name() {
                    "mountedDevice" => attr_map(c, "Dname", "mountedDevice").ok(),
                    _ => None,
                })
                .collect(),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct Component {
    pub vendor: String,
    pub class: String,
    pub group: String,
    pub sub_group: Option<String>,
    pub variant: Option<String>,
    pub version: String,
    pub api_version: Option<String>,
    pub condition: Option<String>,
    pub max_instances: Option<u8>,
    pub is_default: bool,
    pub deprecated: bool,
    pub description: String,
    pub rte_addition: String,
    pub files: Vec<FileRef>,
}

type Components = Vec<Component>;

impl Package {
    pub fn make_components(&self) -> Components {
        self.components
            .0
            .clone()
            .into_iter()
            .map(|comp| Component {
                vendor: comp.vendor.unwrap_or_else(|| self.vendor.clone()),
                class: comp.class.unwrap(),
                group: comp.group.unwrap(),
                sub_group: comp.sub_group,
                variant: comp.variant,
                version: comp
                    .version
                    .unwrap_or_else(|| self.releases.latest_release().version.clone()),
                api_version: comp.api_version,
                condition: comp.condition,
                max_instances: comp.max_instances,
                is_default: comp.is_default,
                deprecated: comp.deprecated,
                description: comp.description,
                rte_addition: comp.rte_addition,
                files: comp.files,
            })
            .collect()
    }

    pub fn make_condition_lookup<'a>(&'a self) -> HashMap<&'a str, &'a Condition> {
        let mut map = HashMap::with_capacity(self.conditions.0.iter().count());
        for cond in self.conditions.0.iter() {
            if let Some(dup) = map.insert(cond.id.as_str(), cond) {
                log::warn!("Duplicate Condition found {}", dup.id);
            }
        }
        map
    }

    pub fn make_dump_devices<'a>(&'a self) -> Vec<(&'a str, DumpDevice<'a>)> {
        let from_pack = FromPack::new(
            &self.vendor,
            &self.name,
            &self.releases.latest_release().version,
            &self.url,
        );
        self.devices
            .0
            .iter()
            .map(|(name, d)| (name.as_str(), DumpDevice::from_device(d, from_pack.clone())))
            .collect()
    }
}
pub fn dump_devices<'a, P: AsRef<Path>, I: IntoIterator<Item = &'a Package>>(
    pdscs: I,
    device_dest: Option<P>,
    board_dest: Option<P>,
) -> Result<(), Error> {
    let pdscs: Vec<&Package> = pdscs.into_iter().collect();
    let devices = pdscs
        .iter()
        .flat_map(|pdsc| pdsc.make_dump_devices().into_iter())
        .collect::<HashMap<_, _>>();
    match device_dest {
        Some(to_file) => {
            if !devices.is_empty() {
                let mut file_contents = Vec::new();
                let mut old_devices: HashMap<&str, DumpDevice> = HashMap::new();
                if let Ok(mut fd) = OpenOptions::new().read(true).open(to_file.as_ref()) {
                    fd.read_to_end(&mut file_contents)?;
                    old_devices = serde_json::from_slice(&file_contents).unwrap_or_default();
                }
                let mut all_devices = BTreeMap::new();
                all_devices.extend(old_devices.iter());
                all_devices.extend(devices.iter());
                let mut options = OpenOptions::new();
                options.write(true);
                options.create(true);
                options.truncate(true);
                if let Ok(fd) = options.open(to_file.as_ref()) {
                    serde_json::to_writer_pretty(fd, &all_devices).unwrap();
                } else {
                    println!("Could not open file {:?}", to_file.as_ref());
                }
            }
        }
        None => println!("{}", &serde_json::to_string_pretty(&devices).unwrap()),
    }
    let boards = pdscs
        .iter()
        .flat_map(|pdsc| pdsc.boards.iter())
        .map(|b| (&b.name, b))
        .collect::<HashMap<_, _>>();
    match board_dest {
        Some(to_file) => {
            let mut file_contents = Vec::new();
            let mut old_boards: HashMap<String, Board> = HashMap::new();
            if let Ok(mut fd) = OpenOptions::new().read(true).open(to_file.as_ref()) {
                fd.read_to_end(&mut file_contents)?;
                old_boards = serde_json::from_slice(&file_contents).unwrap_or_default();
            }
            let mut all_boards = BTreeMap::new();
            all_boards.extend(old_boards.iter());
            all_boards.extend(boards.iter());
            let mut options = OpenOptions::new();
            options.write(true);
            options.create(true);
            options.truncate(true);
            if let Ok(fd) = options.open(to_file.as_ref()) {
                serde_json::to_writer_pretty(fd, &all_boards).unwrap();
            } else {
                println!("Could not open file {:?}", to_file.as_ref());
            }
        }
        None => println!("{}", &serde_json::to_string_pretty(&devices).unwrap()),
    }
    Ok(())
}

pub fn dumps_components<'a, I>(pdscs: I) -> Result<String, Error>
where
    I: IntoIterator<Item = &'a Package>,
{
    let components = pdscs
        .into_iter()
        .flat_map(|pdsc| pdsc.make_components().into_iter())
        .collect::<Vec<_>>();
    Ok(serde_json::to_string_pretty(&components)?)
}
