use std::path::Path;
use std::sync::{Arc, Mutex};
use std::io::Stdout;
use failure::Error;
use clap::{ArgMatches, App, Arg, SubCommand};
use pbr::ProgressBar;

use cmsis_pack::cmsis_update::{install, update, DownloadProgress};
use cmsis_pack::pdsc::{dump_devices, Component, FileRef, Package};
use cmsis_pack::utils::FromElem;

mod config;

pub use config::Config;

struct CliProgress(Arc<Mutex<ProgressBar<Stdout>>>);

impl DownloadProgress for CliProgress {
    fn size(&self, files: usize) {
        if let Ok(mut inner) = self.0.lock() {
            inner.total = files as u64;
            inner.show_speed = false;
            inner.show_bar = true;
        }
    }
    fn progress(&self, _: usize) {}
    fn complete(&self) {
        if let Ok(mut inner) = self.0.lock() {
            inner.inc();
        }
    }
    fn for_file(&self, _: &str) -> Self {
        CliProgress(self.0.clone())
    }
}

impl CliProgress {
    fn new() -> Self {
        let mut progress = ProgressBar::new(363);
        progress.show_speed = false;
        progress.show_time_left = false;
        progress.format("[#> ]");
        progress.message("Downloading Packs ");
        CliProgress(Arc::new(Mutex::new(progress)))
    }
}

pub fn install_args() -> App<'static, 'static> {
    SubCommand::with_name("install")
        .about("Install a CMSIS Pack file")
        .version("0.1.0")
        .arg(
            Arg::with_name("PDSC")
                .required(true)
                .takes_value(true)
                .index(1)
                .multiple(true)
        )
}

pub fn install_command<'a>(
    conf: &Config,
    args: &ArgMatches<'a>,
) -> Result<(), Error> {
    let pdsc_list: Vec<_> = args.values_of("PDSC")
        .unwrap()
        .filter_map(|input| Package::from_path(Path::new(input)).ok())
        .collect();
    let progress = CliProgress::new();
    let updated = install(conf, pdsc_list.iter(), progress)?;
    let num_updated = updated.iter().map(|_| 1).sum::<u32>();
    match num_updated {
        0 => {
            log::info!("Already up to date");
        }
        1 => {
            log::info!("Updated 1 package");
        }
        _ => {
            log::info!("Updated {} package", num_updated);
        }
    }
    Ok(())
}

pub fn update_args<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("update")
        .about("Update CMSIS PDSC files for indexing")
        .version("0.1.0")
}

pub fn update_command<'a>(conf: &Config, _: &ArgMatches<'a>) -> Result<(), Error> {
    let vidx_list = conf.read_vidx_list();
    for url in vidx_list.iter() {
        log::info!("Updating registry from `{}`", url);
    }
    let progress = CliProgress::new();
    let updated = update(conf, vidx_list, progress)?;
    let num_updated = updated.iter().map(|_| 1).sum::<u32>();
    match num_updated {
        0 => {
            log::info!("Already up to date");
        }
        1 => {
            log::info!("Updated 1 package");
        }
        _ => {
            log::info!("Updated {} package", num_updated);
        }
    }
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

pub fn dump_devices_command<'a>(
    c: &Config,
    args: &ArgMatches<'a>,
) -> Result<(), Error> {
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
        .flat_map(|filename| match Package::from_path(&filename) {
            Ok(c) => Some(c),
            Err(e) => {
                log::error!("parsing {:?}: {}", filename, e);
                None
            }
        })
        .collect::<Vec<Package>>();
    let to_ret = dump_devices(&pdscs, args.value_of("devices"), args.value_of("boards"));
    log::debug!("exiting");
    to_ret
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

pub fn check_command<'a>(_: &Config, args: &ArgMatches<'a>) -> Result<(), Error> {
    let filename = args.value_of("INPUT").unwrap();
    match Package::from_path(Path::new(filename)) {
        Ok(c) => {
            log::info!("Parsing succedded");
            log::info!("{} Valid Conditions", c.conditions.0.iter().count());
            let cond_lookup = c.make_condition_lookup();
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
                if let Some(ref cond_name) = condition {
                    if cond_lookup.get(cond_name.as_str()).is_none() {
                        log::warn!(
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
                    if let Some(ref cond_name) = condition {
                        if cond_lookup.get(cond_name.as_str()).is_none() {
                            log::warn!(
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
            log::info!("{} Valid Devices", c.devices.0.len());
            log::info!("{} Valid Software Components", num_components);
            log::info!("{} Valid Files References", num_files);
        }
        Err(e) => {
            log::error!("parsing {}: {}", filename, e);
        }
    }
    log::debug!("exiting");
    Ok(())
}

