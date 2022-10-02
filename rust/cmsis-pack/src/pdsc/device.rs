use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use crate::utils::prelude::*;
use anyhow::{format_err, Error};
use minidom::Element;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Core {
    CortexM0,
    CortexM0Plus,
    CortexM1,
    CortexM3,
    CortexM4,
    CortexM7,
    CortexM23,
    CortexM33,
    SC000,
    SC300,
    ARMV8MBL,
    ARMV8MML,
    CortexR4,
    CortexR5,
    CortexR7,
    CortexR8,
    CortexA5,
    CortexA7,
    CortexA8,
    CortexA9,
    CortexA15,
    CortexA17,
    CortexA32,
    CortexA35,
    CortexA53,
    CortexA57,
    CortexA72,
    CortexA73,
}

impl FromStr for Core {
    type Err = Error;
    fn from_str(from: &str) -> Result<Self, Error> {
        match from {
            "Cortex-M0" => Ok(Core::CortexM0),
            "Cortex-M0+" => Ok(Core::CortexM0Plus),
            "Cortex-M1" => Ok(Core::CortexM1),
            "Cortex-M3" => Ok(Core::CortexM3),
            "Cortex-M4" => Ok(Core::CortexM4),
            "Cortex-M7" => Ok(Core::CortexM7),
            "Cortex-M23" => Ok(Core::CortexM23),
            "Cortex-M33" => Ok(Core::CortexM33),
            "SC000" => Ok(Core::SC000),
            "SC300" => Ok(Core::SC300),
            "ARMV8MBL" => Ok(Core::ARMV8MBL),
            "ARMV8MML" => Ok(Core::ARMV8MML),
            "Cortex-R4" => Ok(Core::CortexR4),
            "Cortex-R5" => Ok(Core::CortexR5),
            "Cortex-R7" => Ok(Core::CortexR7),
            "Cortex-R8" => Ok(Core::CortexR8),
            "Cortex-A5" => Ok(Core::CortexA5),
            "Cortex-A7" => Ok(Core::CortexA7),
            "Cortex-A8" => Ok(Core::CortexA8),
            "Cortex-A9" => Ok(Core::CortexA9),
            "Cortex-A15" => Ok(Core::CortexA15),
            "Cortex-A17" => Ok(Core::CortexA17),
            "Cortex-A32" => Ok(Core::CortexA32),
            "Cortex-A35" => Ok(Core::CortexA35),
            "Cortex-A53" => Ok(Core::CortexA53),
            "Cortex-A57" => Ok(Core::CortexA57),
            "Cortex-A72" => Ok(Core::CortexA72),
            "Cortex-A73" => Ok(Core::CortexA73),
            unknown => Err(format_err!("Unknown core {}", unknown)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FPU {
    None,
    SinglePrecision,
    DoublePrecision,
}

impl FromStr for FPU {
    type Err = Error;
    fn from_str(from: &str) -> Result<Self, Error> {
        match from {
            "FPU" => Ok(FPU::SinglePrecision),
            "SP_FPU" => Ok(FPU::SinglePrecision),
            "1" => Ok(FPU::SinglePrecision),
            "None" => Ok(FPU::None),
            "0" => Ok(FPU::None),
            "DP_FPU" => Ok(FPU::DoublePrecision),
            "2" => Ok(FPU::DoublePrecision),
            unknown => Err(format_err!("Unknown fpu {}", unknown)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MPU {
    NotPresent,
    Present,
}

impl FromStr for MPU {
    type Err = Error;
    fn from_str(from: &str) -> Result<Self, Error> {
        match from {
            "MPU" => Ok(MPU::Present),
            "1" => Ok(MPU::Present),
            "None" => Ok(MPU::NotPresent),
            "0" => Ok(MPU::NotPresent),
            unknown => Err(format_err!("Unknown fpu {}", unknown)),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Processor {
    pub core: Core,
    pub fpu: FPU,
    pub mpu: MPU,
    pub ap: u8,
    pub dp: u8,
    pub apid: Option<u32>,
    pub address: Option<u32>,
    pub svd: Option<String>,
    pub name: Option<String>,
    pub unit: usize,
    pub default_reset_sequence: Option<String>,
}

#[derive(Debug, Clone)]
struct ProcessorBuilder {
    core: Option<Core>,
    units: Option<usize>,
    name: Option<String>,
    fpu: Option<FPU>,
    mpu: Option<MPU>,
}

impl ProcessorBuilder {
    fn merge(self, other: &Self) -> Self {
        Self {
            core: self.core.or(other.core.clone()),
            units: self.units.or(other.units),
            name: self.name.or(other.name.clone()),
            fpu: self.fpu.or(other.fpu.clone()),
            mpu: self.mpu.or(other.mpu.clone()),
        }
    }
    fn build(self, debugs: &Vec<Debug>) -> Result<Vec<Processor>, Error> {
        let units = self.units.unwrap_or(1);
        let name = self.name.clone();

        let map = (0..units)
            .into_iter()
            .map(|unit| {
                let default = Debug {
                    ap: 0,
                    dp: 0,
                    apid: None,
                    address: None,
                    svd: None,
                    name: name.clone(),
                    unit: None,
                    default_reset_sequence: None,
                };
                let Debug {
                    ap,
                    dp,
                    apid,
                    address,
                    svd,
                    name,
                    default_reset_sequence,
                    ..
                } = debugs
                    .iter()
                    .find(|debug| {
                        // If the <debug> element has a specific unit attribute we compare by that as well.
                        // If not we just compare the name.
                        let unit_condition = debug.unit.map(|u| u == unit).unwrap_or(true);
                        debug.name == name && unit_condition
                    })
                    .unwrap_or_else(|| &default);

                Ok(Processor {
                    core: self
                        .core
                        .clone()
                        .ok_or_else(|| format_err!("No Core found!"))?,
                    fpu: self.fpu.clone().unwrap_or(FPU::None),
                    mpu: self.mpu.clone().unwrap_or(MPU::NotPresent),
                    ap: *ap,
                    dp: *dp,
                    apid: *apid,
                    address: *address,
                    svd: svd.clone(),
                    name: name.clone(),
                    unit,
                    default_reset_sequence: default_reset_sequence.clone(),
                })
            })
            .collect::<Result<Vec<_>, _>>();

        map
    }
}

impl FromElem for ProcessorBuilder {
    fn from_elem(e: &Element) -> Result<Self, Error> {
        Ok(ProcessorBuilder {
            core: attr_parse(e, "Dcore", "processor").ok(),
            units: attr_parse(e, "Punits", "processor").ok(),
            fpu: attr_parse(e, "Dfpu", "processor").ok(),
            mpu: attr_parse(e, "Dmpu", "processor").ok(),
            name: attr_parse(e, "Pname", "processor").ok(),
        })
    }
}

#[derive(Debug, Clone)]
struct ProcessorsBuilder(Vec<ProcessorBuilder>);

impl ProcessorsBuilder {
    fn merge(mut self, parent: &Option<Self>) -> Result<Self, Error> {
        if let Some(parent) = parent {
            if let [parent] = &parent.0[..] {
                self.0 = self.0.into_iter().map(|x| x.merge(parent)).collect();
            } else {
                Err(format_err!(
                    "Support for two parent processors not implemented!"
                ))?;
            }
        }
        Ok(self)
    }

    fn merge_into(&mut self, other: Self) {
        self.0.extend(other.0.into_iter());
    }

    fn build(self, debugs: Vec<Debug>) -> Result<Vec<Processor>, Error> {
        let mut vec = vec![];
        for processor in self.0.into_iter() {
            vec.extend(processor.build(&debugs)?);
        }
        Ok(vec)
    }
}

impl FromElem for ProcessorsBuilder {
    fn from_elem(e: &Element) -> Result<Self, Error> {
        Ok(ProcessorsBuilder(vec![ProcessorBuilder::from_elem(e)?]))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Debug {
    pub ap: u8,
    pub dp: u8,
    pub apid: Option<u32>,
    pub address: Option<u32>,
    pub svd: Option<String>,
    pub name: Option<String>,
    pub unit: Option<usize>,
    pub default_reset_sequence: Option<String>,
}

#[derive(Debug, Clone)]
struct DebugBuilder {
    ap: Option<u8>,
    dp: Option<u8>,
    apid: Option<u32>,
    address: Option<u32>,
    svd: Option<String>,
    name: Option<String>,
    unit: Option<usize>,
    default_reset_sequence: Option<String>,
}

impl DebugBuilder {
    fn build(self) -> Debug {
        Debug {
            ap: self.ap.unwrap_or_default(),
            dp: self.dp.unwrap_or_default(),
            apid: self.apid,
            address: self.address,
            svd: self.svd,
            name: self.name,
            unit: self.unit,
            default_reset_sequence: self.default_reset_sequence,
        }
    }
}

impl FromElem for DebugBuilder {
    fn from_elem(e: &Element) -> Result<Self, Error> {
        Ok(DebugBuilder {
            ap: attr_parse(e, "__ap", "debug").ok(),
            dp: attr_parse(e, "__dp", "debug").ok(),
            apid: attr_parse(e, "__apid", "debug").ok(),
            address: attr_parse(e, "address", "debug").ok(),
            svd: attr_parse(e, "svd", "debug").ok(),
            name: attr_parse(e, "Pname", "debug").ok(),
            unit: attr_parse(e, "Punit", "debug").ok(),
            default_reset_sequence: attr_parse(e, "defaultResetSequence", "debug").ok(),
        })
    }
}

#[derive(Debug)]
struct DebugsBuilder(Vec<DebugBuilder>);

impl FromElem for DebugsBuilder {
    fn from_elem(e: &Element) -> Result<Self, Error> {
        Ok(DebugsBuilder(vec![DebugBuilder::from_elem(e)?]))
    }
}

impl DebugsBuilder {
    fn merge(mut self, parent: &Self) -> Self {
        self.0.extend(parent.0.iter().map(|v| v.clone()));
        self
    }

    fn merge_into(&mut self, other: Self) {
        self.0.extend(other.0.into_iter())
    }

    fn build(self) -> Vec<Debug> {
        self.0.into_iter().map(|debug| debug.build()).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPermissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub peripheral: bool,
    pub secure: bool,
    pub non_secure: bool,
    pub non_secure_callable: bool,
}

impl MemoryPermissions {
    fn from_str(input: &str) -> Self {
        let mut ret = MemoryPermissions {
            read: false,
            write: false,
            execute: false,
            peripheral: false,
            secure: false,
            non_secure: false,
            non_secure_callable: false,
        };
        for c in input.chars() {
            match c {
                'r' => ret.read = true,
                'w' => ret.write = true,
                'x' => ret.execute = true,
                'p' => ret.peripheral = true,
                's' => ret.secure = true,
                'n' => ret.non_secure = true,
                'c' => ret.non_secure_callable = true,
                _ => (),
            }
        }
        ret
    }
}

enum NumberBool {
    False,
    True,
}

impl Into<bool> for NumberBool {
    fn into(self) -> bool {
        match self {
            NumberBool::True => true,
            NumberBool::False => false,
        }
    }
}

impl FromStr for NumberBool {
    type Err = Error;
    fn from_str(from: &str) -> Result<Self, Error> {
        match from {
            "true" => Ok(NumberBool::True),
            "1" => Ok(NumberBool::True),
            "false" => Ok(NumberBool::False),
            "0" => Ok(NumberBool::False),
            unknown => Err(format_err!(
                "unkown boolean found in merory startup {}",
                unknown
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub p_name: Option<String>,
    pub access: MemoryPermissions,
    pub start: u64,
    pub size: u64,
    pub startup: bool,
    pub default: bool,
}

struct MemElem(String, Memory);

impl FromElem for MemElem {
    fn from_elem(e: &Element) -> Result<Self, Error> {
        let access = MemoryPermissions::from_str(e.attr("access").unwrap_or_else(|| {
            let memtype = e.attr("id").unwrap_or_default();
            if memtype.contains("ROM") {
                "rx"
            } else if memtype.contains("RAM") {
                "rw"
            } else {
                ""
            }
        }));
        let name = e
            .attr("id")
            .or_else(|| e.attr("name"))
            .map(|s| s.to_string())
            .ok_or_else(|| format_err!("No name found for memory"))?;
        let p_name = e.attr("Pname").map(|s| s.to_string());
        let start = attr_parse_hex(e, "start", "memory")?;
        let size = attr_parse_hex(e, "size", "memory")?;
        let startup = attr_parse(e, "startup", "memory")
            .map(|nb: NumberBool| nb.into())
            .unwrap_or_default();
        let default = attr_parse(e, "default", "memory")
            .map(|nb: NumberBool| nb.into())
            .unwrap_or_default();
        Ok(MemElem(
            name,
            Memory {
                p_name,
                access,
                start,
                size,
                startup,
                default,
            },
        ))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Memories(pub HashMap<String, Memory>);

fn merge_memories(lhs: Memories, rhs: &Memories) -> Memories {
    let rhs: Vec<_> = rhs
        .0
        .iter()
        .filter_map(|(k, v)| {
            if lhs.0.contains_key(k) {
                None
            } else {
                Some((k.clone(), v.clone()))
            }
        })
        .collect();
    let mut lhs = lhs;
    lhs.0.extend(rhs);
    lhs
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Algorithm {
    pub file_name: PathBuf,
    pub start: u64,
    pub size: u64,
    pub default: bool,
    pub ram_start: Option<u64>,
    pub ram_size: Option<u64>,
}

impl FromElem for Algorithm {
    fn from_elem(e: &Element) -> Result<Self, Error> {
        let default = attr_parse(e, "default", "memory")
            .map(|nb: NumberBool| nb.into())
            .unwrap_or_default();

        let file_name: &str = attr_map(e, "name", "algorithm")?;
        Ok(Self {
            file_name: file_name.replace("\\", "/").into(),
            start: attr_parse_hex(e, "start", "algorithm")?,
            size: attr_parse_hex(e, "size", "algorithm")?,
            ram_start: attr_parse_hex(e, "RAMstart", "algorithm").ok(),
            ram_size: attr_parse_hex(e, "RAMsize", "algorithm").ok(),
            default,
        })
    }
}

#[derive(Debug)]
struct DeviceBuilder<'dom> {
    name: Option<&'dom str>,
    algorithms: Vec<Algorithm>,
    memories: Memories,
    processor: Option<ProcessorsBuilder>,
    debugs: DebugsBuilder,
    vendor: Option<&'dom str>,
    family: Option<&'dom str>,
    sub_family: Option<&'dom str>,
}

#[derive(Debug, Serialize)]
pub struct Device {
    pub name: String,
    pub memories: Memories,
    pub algorithms: Vec<Algorithm>,
    pub processors: Vec<Processor>,
    pub vendor: Option<String>,
    pub family: String,
    pub sub_family: Option<String>,
}

impl<'dom> DeviceBuilder<'dom> {
    fn from_elem(e: &'dom Element) -> Self {
        let memories = Memories(HashMap::new());
        let mut family = None;
        let mut sub_family = None;
        if e.name() == "family" {
            family = e.attr("Dfamily");
        }
        if e.name() == "subFamily" {
            sub_family = e.attr("DsubFamily");
        }
        DeviceBuilder {
            name: e.attr("Dname").or_else(|| e.attr("Dvariant")),
            vendor: e.attr("Dvendor"),
            memories,
            algorithms: Vec::new(),
            processor: None,
            debugs: DebugsBuilder(Vec::new()),
            family,
            sub_family,
        }
    }

    fn build(self) -> Result<Device, Error> {
        let name = self
            .name
            .map(|s| s.into())
            .ok_or_else(|| format_err!("Device found without a name"))?;
        let family = self
            .family
            .map(|s| s.into())
            .ok_or_else(|| format_err!("Device found without a family"))?;

        let debugs = self.debugs.build();

        let processors = match self.processor {
            Some(pb) => pb.build(debugs)?,
            None => return Err(format_err!("Device found without a processor {}", name)),
        };

        Ok(Device {
            processors,
            name,
            memories: self.memories,
            algorithms: self.algorithms,
            vendor: self.vendor.map(str::to_string),
            family,
            sub_family: self.sub_family.map(str::to_string),
        })
    }

    fn add_parent(mut self, parent: &Self) -> Result<Self, Error> {
        self.algorithms.extend_from_slice(&parent.algorithms);
        Ok(Self {
            name: self.name.or(parent.name),
            algorithms: self.algorithms,
            memories: merge_memories(self.memories, &parent.memories),
            processor: match self.processor {
                Some(old_proc) => Some(old_proc.merge(&parent.processor)?),
                None => parent.processor.clone(),
            },
            debugs: self.debugs.merge(&parent.debugs),
            vendor: self.vendor.or(parent.vendor),
            family: self.family.or(parent.family),
            sub_family: self.sub_family.or(parent.sub_family),
        })
    }

    fn add_processor(&mut self, processor: ProcessorsBuilder) -> &mut Self {
        match self.processor {
            None => self.processor = Some(processor),
            Some(ref mut origin) => origin.merge_into(processor),
        };
        self
    }

    fn add_debug(&mut self, debug: DebugsBuilder) -> &mut Self {
        self.debugs.merge_into(debug);
        self
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

fn parse_device<'dom>(e: &'dom Element) -> Vec<DeviceBuilder<'dom>> {
    let mut device = DeviceBuilder::from_elem(e);
    let variants = e
        .children()
        .filter_map(|child| match child.name() {
            "variant" => Some(DeviceBuilder::from_elem(child)),
            "memory" => {
                FromElem::from_elem(child)
                    .ok_warn()
                    .map(|mem| device.add_memory(mem));
                None
            }
            "algorithm" => {
                FromElem::from_elem(child)
                    .ok_warn()
                    .map(|alg| device.add_algorithm(alg));
                None
            }
            "processor" => {
                FromElem::from_elem(child)
                    .ok_warn()
                    .map(|prc| device.add_processor(prc));
                None
            }
            "debug" => {
                FromElem::from_elem(child)
                    .ok_warn()
                    .map(|debug| device.add_debug(debug));
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
            .flat_map(|bld| bld.add_parent(&device).ok_warn())
            .collect()
    }
}

fn parse_sub_family<'dom>(e: &'dom Element) -> Vec<DeviceBuilder<'dom>> {
    let mut sub_family_device = DeviceBuilder::from_elem(e);
    let devices = e
        .children()
        .flat_map(|child| match child.name() {
            "device" => parse_device(child),
            "memory" => {
                FromElem::from_elem(child)
                    .ok_warn()
                    .map(|mem| sub_family_device.add_memory(mem));
                Vec::new()
            }
            "algorithm" => {
                FromElem::from_elem(child)
                    .ok_warn()
                    .map(|alg| sub_family_device.add_algorithm(alg));
                Vec::new()
            }
            "processor" => {
                FromElem::from_elem(child)
                    .ok_warn()
                    .map(|prc| sub_family_device.add_processor(prc));
                Vec::new()
            }
            _ => Vec::new(),
        })
        .collect::<Vec<_>>();
    devices
        .into_iter()
        .flat_map(|bldr| bldr.add_parent(&sub_family_device).ok_warn())
        .collect()
}

fn parse_family(e: &Element) -> Result<Vec<Device>, Error> {
    let mut family_device = DeviceBuilder::from_elem(e);
    let all_devices = e
        .children()
        .flat_map(|child| match child.name() {
            "subFamily" => parse_sub_family(child),
            "device" => parse_device(child),
            "memory" => {
                FromElem::from_elem(child)
                    .ok_warn()
                    .map(|mem| family_device.add_memory(mem));
                Vec::new()
            }
            "algorithm" => {
                FromElem::from_elem(child)
                    .ok_warn()
                    .map(|alg| family_device.add_algorithm(alg));
                Vec::new()
            }
            "processor" => {
                FromElem::from_elem(child)
                    .ok_warn()
                    .map(|prc| family_device.add_processor(prc));
                Vec::new()
            }
            _ => Vec::new(),
        })
        .collect::<Vec<_>>();
    all_devices
        .into_iter()
        .map(|bldr| bldr.add_parent(&family_device).and_then(|dev| dev.build()))
        .collect()
}

#[derive(Default, Serialize)]
pub struct Devices(pub HashMap<String, Device>);

impl FromElem for Devices {
    fn from_elem(e: &Element) -> Result<Self, Error> {
        e.children()
            .fold(Ok(HashMap::new()), |res, c| match (res, parse_family(c)) {
                (Ok(mut devs), Ok(add_this)) => {
                    devs.extend(add_this.into_iter().map(|dev| (dev.name.clone(), dev)));
                    Ok(devs)
                }
                (Ok(_), Err(e)) => Err(e),
                (Err(e), Ok(_)) => Err(e),
                (Err(e), Err(_)) => Err(e),
            })
            .map(Devices)
    }
}
