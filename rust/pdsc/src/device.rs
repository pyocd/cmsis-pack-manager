use std::collections::HashMap;
use std::path::PathBuf;

use minidom::{Error, ErrorKind, Element};
use slog::Logger;

use utils::parse::{attr_map, attr_parse, attr_parse_hex, FromElem};
use utils::ResultLogExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemoryPermissions {
    read: bool,
    write: bool,
    execute: bool,
}

impl MemoryPermissions {
    fn from_str(input: &str) -> Self {
        let mut ret = MemoryPermissions {
            read: false,
            write: false,
            execute: false,
        };
        for c in input.chars() {
            match c {
                'r' => ret.read = true,
                'w' => ret.write = true,
                'x' => ret.execute = true,
                _ => (),
            }
        }
        ret
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Memory {
    access: MemoryPermissions,
    start: u64,
    size: u64,
    startup: bool,
}

struct MemElem(String, Memory);

impl FromElem for MemElem {
    fn from_elem(e: &Element, _l: &Logger) -> Result<Self, Error> {
        let access = e.attr("id")
            .map(|memtype| if memtype.contains("ROM") {
                "rx"
            } else if memtype.contains("RAM") {
                "rw"
            } else {
                ""
            })
            .or_else(|| e.attr("access"))
            .map(|memtype| MemoryPermissions::from_str(memtype))
            .unwrap();
        let name = e.attr("id")
            .or_else(|| e.attr("name"))
            .map(|s| s.to_string())
            .ok_or_else(|| err_msg!("No name found for memory"))?;
        let start = attr_parse_hex(e, "start", "memory")?;
        let size = attr_parse_hex(e, "size", "memory")?;
        let startup = attr_parse(e, "startup", "memory").unwrap_or_default();
        Ok(MemElem(
            name,
            Memory {
                access,
                start,
                size,
                startup,
            },
        ))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Memories(HashMap<String, Memory>);

fn merge_memories(lhs: Memories, rhs: &Memories) -> Memories {
    let rhs: Vec<_> = rhs.0
        .iter()
        .filter_map(|(k, v)| if lhs.0.contains_key(k) {
            None
        } else {
            Some((k.clone(), v.clone()))
        })
        .collect();
    let mut lhs = lhs;
    lhs.0.extend(rhs);
    lhs
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Algorithm {
    file_name: PathBuf,
    start: u64,
    size: u64,
    default: bool,
}

impl FromElem for Algorithm {
    fn from_elem(e: &Element, _l: &Logger) -> Result<Self, Error> {
        Ok(Self {
            file_name: attr_map(e, "name", "algorithm")?,
            start: attr_parse_hex(e, "start", "algorithm")?,
            size: attr_parse_hex(e, "size", "algorithm")?,
            default: attr_parse(e, "default", "algorithm").unwrap_or_default(),
        })
    }
}

#[derive(Debug)]
struct DeviceBuilder<'dom> {
    name: Option<&'dom str>,
    algorithms: Vec<Algorithm>,
    memories: Memories,
}

#[derive(Debug, Serialize)]
pub struct Device {
    pub name: String,
    pub memories: Memories,
    pub algorithms: Vec<Algorithm>,
}

impl<'dom> DeviceBuilder<'dom> {
    fn from_elem(e: &'dom Element) -> Self {
        let memories = Memories(HashMap::new());
        let bldr = DeviceBuilder {
            name: e.attr("Dname").or_else(|| e.attr("Dvariant")),
            memories,
            algorithms: Vec::new(),
        };
        bldr
    }

    fn build(self) -> Result<Device, Error> {
        Ok(Device {
            name: self.name.map(|s| s.into()).ok_or_else(|| {
                err_msg!("Device found without a name")
            })?,
            memories: self.memories,
            algorithms: self.algorithms,
        })
    }

    fn add_parent(mut self, parent: &Self) -> Self {
        self.algorithms.extend_from_slice(&parent.algorithms);
        Self {
            name: self.name.or(parent.name),
            algorithms: self.algorithms,
            memories: merge_memories(self.memories, &parent.memories),
        }
    }

    fn add_memory(&mut self, MemElem(name, mem): MemElem) -> &mut Self {
        self.memories.0.insert(name, mem);
        self
    }

    fn add_algorithm(&mut self, alg: Algorithm) -> &mut Self {
        self.algorithms.push(alg);
        self
    }
}

fn parse_device<'dom>(e: &'dom Element, l: &Logger) -> Vec<DeviceBuilder<'dom>> {
    let mut device = DeviceBuilder::from_elem(e);
    let variants = e.children()
        .filter_map(|child| match child.name() {
            "variant" => Some(DeviceBuilder::from_elem(child)),
            "memory" => {
                FromElem::from_elem(child, l)
                    .ok_warn(l)
                    .map(|mem| device.add_memory(mem));
                None
            }
            "algorithm" => {
                FromElem::from_elem(child, l)
                    .ok_warn(l)
                    .map(|alg| device.add_algorithm(alg));
                None
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if variants.is_empty() {
        vec![device]
    } else {
        variants
            .into_iter()
            .map(|bld| bld.add_parent(&device))
            .collect()
    }
}

fn parse_sub_family<'dom>(e: &'dom Element, l: &Logger) -> Vec<DeviceBuilder<'dom>> {
    let mut sub_family_device = DeviceBuilder::from_elem(e);
    let devices = e.children()
        .flat_map(|child| match child.name() {
            "device" => parse_device(child, l),
            "memory" => {
                FromElem::from_elem(child, l)
                    .ok_warn(l)
                    .map(|mem| sub_family_device.add_memory(mem));
                Vec::new()
            }
            "algorithm" => {
                FromElem::from_elem(child, l)
                    .ok_warn(l)
                    .map(|alg| sub_family_device.add_algorithm(alg));
                Vec::new()
            }
            _ => Vec::new(),
        })
        .collect::<Vec<_>>();
    devices
        .into_iter()
        .map(|bldr| bldr.add_parent(&sub_family_device))
        .collect()
}

fn parse_family<'dom>(e: &Element, l: &Logger) -> Result<Vec<Device>, Error> {
    let mut family_device = DeviceBuilder::from_elem(e);
    let all_devices = e.children()
        .flat_map(|child| match child.name() {
            "subFamily" => parse_sub_family(child, &l),
            "device" => parse_device(child, &l),
            "memory" => {
                FromElem::from_elem(child, l)
                    .ok_warn(l)
                    .map(|mem| family_device.add_memory(mem));
                Vec::new()
            }
            "algorithm" => {
                FromElem::from_elem(child, l)
                    .ok_warn(l)
                    .map(|alg| family_device.add_algorithm(alg));
                Vec::new()
            }
            _ => Vec::new(),
        })
        .collect::<Vec<_>>();
    all_devices
        .into_iter()
        .map(|bldr| bldr.add_parent(&family_device).build())
        .collect()
}

#[derive(Default, Serialize)]
pub struct Devices(pub(crate) HashMap<String, Device>);

impl FromElem for Devices {
    fn from_elem(e: &Element, l: &Logger) -> Result<Self, Error> {
        e.children()
            .fold(
                Ok(HashMap::new()),
                |res, c| match (res, parse_family(c, l)) {
                    (Ok(mut devs), Ok(add_this)) => {
                        devs.extend(add_this.into_iter().map(|dev| (dev.name.clone(), dev)));
                        Ok(devs)
                    }
                    (Ok(_), Err(e)) => Err(e),
                    (Err(e), Ok(_)) => Err(e),
                    (Err(e), Err(_)) => Err(e),
                },
            )
            .map(Devices)
    }
}
