#[macro_use]
extern crate utils;
#[macro_use]
extern crate slog;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate failure;

extern crate pack_index;
extern crate clap;
extern crate minidom;

use std::borrow::Cow;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::Path;
use std::collections::{HashMap, BTreeMap};
use minidom::{Element, Error, ErrorKind};
use clap::{App, Arg, ArgMatches, SubCommand};
use slog::Logger;

use utils::parse::{assert_root_name, attr_map, child_text, get_child_no_ns, FromElem};
use utils::ResultLogExt;
use pack_index::config::Config;
use failure::Error as FailError;

mod component;
mod condition;
mod device;
pub use component::{ComponentBuilders, FileRef};
pub use condition::{Condition, Conditions};
pub use device::{Device, Devices, Memories, Algorithm};

pub struct Release {
    pub version: String,
    pub text: String,
}

impl FromElem for Release {
    fn from_elem(e: &Element, _: &Logger) -> Result<Self, Error> {
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
    fn from_elem(e: &Element, l: &Logger) -> Result<Self, Error> {
        assert_root_name(e, "releases")?;
        let to_ret: Vec<_> = e.children()
            .flat_map(|c| Release::from_elem(c, l).ok_warn(l))
            .collect();
        if to_ret.len() == 0usize {
            Err(err_msg!("There must be at least one release!"))
        } else {
            Ok(Releases(to_ret))
        }
    }
}


#[derive(Debug, Serialize, Deserialize)]
struct DumpDevice<'a> {
    name: &'a str,
    memories: Cow<'a, Memories>,
    algorithms: Cow<'a, Vec<Algorithm>>,
    from_pack: FromPack<'a>,
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
            from_pack: from_pack,
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
    conditions: Conditions,
    devices: Devices,
    pub boards: Vec<Board>,
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
        let components = get_child_no_ns(e, "components")
            .and_then(|c| ComponentBuilders::from_elem(c, &l).ok_warn(&l))
            .unwrap_or_default();
        let releases = get_child_no_ns(e, "releases")
            .and_then(|c| Releases::from_elem(c, &l).ok_warn(&l))
            .unwrap_or_default();
        let conditions = get_child_no_ns(e, "conditions")
            .and_then(|c| Conditions::from_elem(c, &l).ok_warn(&l))
            .unwrap_or_default();
        let devices = get_child_no_ns(e, "devices")
            .and_then(|c| Devices::from_elem(c, &l).ok_warn(&l))
            .unwrap_or_default();
        let boards = get_child_no_ns(e, "boards")
            .map(|c| Board::vec_from_children(c.children(), &l))
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

#[derive(Debug, Serialize)]
pub struct Board {
    name: String,
    mounted_devices: Vec<String>,
}

impl FromElem for Board {
    fn from_elem(e: &Element, _: &Logger) -> Result<Self, Error> {
        Ok(Self {
            name: attr_map(e, "name", "board")?,
            mounted_devices: e.children()
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
        self.components
            .0
            .clone()
            .into_iter()
            .map(|comp| {
                Component {
                    vendor: comp.vendor.unwrap_or_else(|| self.vendor.clone()),
                    class: comp.class.unwrap(),
                    group: comp.group.unwrap(),
                    sub_group: comp.sub_group,
                    variant: comp.variant,
                    version: comp.version.unwrap_or_else(|| {
                        self.releases.latest_release().version.clone()
                    }),
                    api_version: comp.api_version,
                    condition: comp.condition,
                    max_instances: comp.max_instances,
                    is_default: comp.is_default,
                    deprecated: comp.deprecated,
                    description: comp.description,
                    rte_addition: comp.rte_addition,
                    files: comp.files,
                }
            })
            .collect()
    }

    fn make_condition_lookup<'a>(&'a self, l: &Logger) -> HashMap<&'a str, &'a Condition> {
        let mut map = HashMap::with_capacity(self.conditions.0.iter().count());
        for cond in self.conditions.0.iter() {
            if let Some(dup) = map.insert(cond.id.as_str(), cond) {
                warn!(l, "Duplicate Condition found {}", dup.id);
            }
        }
        map
    }

    fn make_dump_devices<'a>(&'a self) -> Vec<(&'a str, DumpDevice<'a>)> {
        let from_pack = FromPack::new(
            &self.vendor,
            &self.name,
            &self.releases.latest_release().version,
            &self.url,
        );
        self.devices
            .0
            .iter()
            .map(|(name, d)| {
                (name.as_str(), DumpDevice::from_device(d, from_pack.clone()))
            })
            .collect()

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

pub fn check_command<'a>(_: &Config, args: &ArgMatches<'a>, l: &Logger) -> Result<(), FailError> {
    let filename = args.value_of("INPUT").unwrap();
    match Package::from_path(Path::new(filename.clone()), &l) {
        Ok(c) => {
            info!(l, "Parsing succedded");
            info!(l, "{} Valid Conditions", c.conditions.0.iter().count());
            let cond_lookup = c.make_condition_lookup(l);
            let mut num_components = 0;
            let mut num_files = 0;
            for &Component {
                ref class,
                ref group,
                ref condition,
                ref files,
                ..
            } in c.make_components().iter()
            {
                num_components += 1;
                num_files += files.iter().count();
                if let &Some(ref cond_name) = condition {
                    if cond_lookup.get(cond_name.as_str()).is_none() {
                        warn!(
                            l,
                            "Component {}::{} references an unknown condition '{}'",
                            class,
                            group,
                            cond_name
                        );
                    }
                }
                for &FileRef {
                    ref path,
                    ref condition,
                    ..
                } in files.iter()
                {
                    if let &Some(ref cond_name) = condition {
                        if cond_lookup.get(cond_name.as_str()).is_none() {
                            warn!(
                                l,
                                "File {:?} Component {}::{} references an unknown condition '{}'",
                                path,
                                class,
                                group,
                                cond_name
                            );
                        }
                    }
                }
            }
            info!(l, "{} Valid Devices", c.devices.0.len());
            info!(l, "{} Valid Software Components", num_components);
            info!(l, "{} Valid Files References", num_files);
        }
        Err(e) => {
            error!(l, "parsing {}: {}", filename, e);
        }
    }
    debug!(l, "exiting");
    Ok(())
}

pub fn dump_devices_args<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("dump-devices")
        .about("Dump devices as json")
        .version("0.1.0")
        .arg(
            Arg::with_name("devices")
                .short("d")
                .takes_value(true)
                .help("Dump JSON in the specified file"),
        )
        .arg(Arg::with_name("boards").short("b").takes_value(true).help(
            "Dump JSON in the specified file",
        ))
        .arg(
            Arg::with_name("INPUT")
                .help("Input file to dump devices from")
                .index(1),
        )

}

pub fn dump_devices<'a, P: AsRef<Path>, I: IntoIterator<Item = &'a Package>>(
    pdscs: I,
    device_dest: Option<P>,
    board_dest: Option<P>,
    _: &Logger,
) -> Result<(), FailError> {
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
                let mut all_devices = BTreeMap::new();
                if let Ok(mut fd) = OpenOptions::new().read(true).open(to_file.as_ref()) {
                    fd.read_to_end(&mut file_contents)?;
                    old_devices = serde_json::from_slice(&file_contents).unwrap_or_default();
                }
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
            let mut options = OpenOptions::new();
            options.write(true);
            options.create(true);
            options.truncate(true);
            if let Ok(fd) = options.open(to_file.as_ref()) {
                serde_json::to_writer_pretty(fd, &boards).unwrap();
            } else {
                println!("Could not open file {:?}", to_file.as_ref());
            }
        }
        None => println!("{}", &serde_json::to_string_pretty(&devices).unwrap()),
    }
    Ok(())
}

pub fn dump_devices_command<'a>(
    c: &Config,
    args: &ArgMatches<'a>,
    l: &Logger,
) -> Result<(), FailError> {
    let files = args.value_of("INPUT").map(|input| {
        vec![Box::new(Path::new(input)).to_path_buf()]
    });
    let filenames = files
        .or_else(|| {
            c.pack_store.read_dir().ok().map(|rd| {
                rd.flat_map(|dirent| dirent.into_iter().map(|p| p.path()))
                    .collect()
            })
        })
        .unwrap();
    let pdscs = filenames
        .into_iter()
        .flat_map(|filename| match Package::from_path(&filename, &l) {
            Ok(c) => Some(c),
            Err(e) => {
                error!(l, "parsing {:?}: {}", filename, e);
                None
            }
        })
        .collect::<Vec<Package>>();
    let to_ret = dump_devices(&pdscs, args.value_of("devices"), args.value_of("boards"), l);
    debug!(l, "exiting");
    to_ret
}

pub fn dumps_components<'a, I>(pdscs: I) -> Result<String, FailError>
    where I: IntoIterator<Item = &'a Package>,
{
    let components = pdscs
        .into_iter()
        .flat_map(|pdsc| pdsc.make_components().into_iter())
        .collect::<Vec<_>>();
    Ok(serde_json::to_string_pretty(&components)?)
}
