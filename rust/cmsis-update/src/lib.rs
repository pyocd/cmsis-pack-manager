#![feature(proc_macro, conservative_impl_trait, generators, libc)]

extern crate futures_await as futures;
extern crate tokio_core;
extern crate hyper;
extern crate hyper_tls;
extern crate minidom;
extern crate clap;
extern crate failure;
extern crate indicatif;

#[macro_use]
extern crate slog;

extern crate utils;
extern crate pack_index;
extern crate pdsc;

use futures::prelude::*;
use futures::Stream;
use futures::stream::iter_ok;
use hyper::{Body, Client};
use hyper::client::Connect;
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Core;
use std::iter::Iterator;
use std::path::{Path, PathBuf};
use clap::{App, Arg, ArgMatches, SubCommand};
use slog::Logger;
use failure::Error;

use pack_index::config::Config;
use pdsc::Package;
use utils::parse::FromElem;

pub mod upgrade;
mod redirect;
mod vidx;
mod dl_pdsc;
mod dl_pack;

use vidx::{download_vidx_list, flatmap_pdscs};
use dl_pdsc::{download_pdsc_stream};
use dl_pack::{download_pack_stream};

/// Create a future of the update command.
pub fn update_future<'a, C, I>(
    config: &'a Config,
    vidx_list: I,
    client: &'a Client<C, Body>,
    logger: &'a Logger,
    spin: bool,
) -> impl Future<Item = Vec<PathBuf>, Error = Error> + 'a
    where C: Connect,
          I: IntoIterator<Item = String> + 'a,
{
    let parsed_vidx = download_vidx_list(vidx_list, client, logger);
    let pdsc_list = parsed_vidx
        .filter_map(move |vidx| match vidx {
            Ok(v) => Some(flatmap_pdscs(v, client, logger)),
            Err(_) => None,
        })
        .flatten();
    let pdscs = download_pdsc_stream(config, pdsc_list, client, logger, spin);
    pdscs.filter_map(id).collect()
}

fn id<T>(slf: T) -> T {
    slf
}

// This will "trick" the borrow checker into thinking that the lifetimes for
// client and core are at least as big as the lifetime for pdscs, which they actually are
fn update_inner<C, I>(
    config: &Config,
    vidx_list: I,
    core: &mut Core,
    client: &Client<C, Body>,
    logger: &Logger,
    spin: bool,
) -> Result<Vec<PathBuf>, Error>
where
    C: Connect,
    I: IntoIterator<Item = String>,
{
    core.run(update_future(config, vidx_list, client, logger, spin))
}

/// Flatten a list of Vidx Urls into a list of updated CMSIS packs
pub fn update<I>(config: &Config, vidx_list: I, logger: &Logger, spin: bool) -> Result<Vec<PathBuf>, Error>
where
    I: IntoIterator<Item = String>,
{
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let client = Client::configure()
        .keep_alive(true)
        .connector(HttpsConnector::new(4, &handle).unwrap())
        .build(&handle);
    update_inner(config, vidx_list, &mut core, &client, logger, spin)
}

pub fn update_args<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("update")
        .about("Update CMSIS PDSC files for indexing")
        .version("0.1.0")
}

pub fn update_command<'a>(conf: &Config, _: &ArgMatches<'a>, logger: &Logger) -> Result<(), Error> {
    let vidx_list = conf.read_vidx_list(&logger);
    for url in vidx_list.iter() {
        info!(logger, "Updating registry from `{}`", url);
    }
    let updated = update(conf, vidx_list, logger, true)?;
    let num_updated = updated.iter().map(|_| 1).sum::<u32>();
    match num_updated {
        0 => {
            info!(logger, "Already up to date");
        }
        1 => {
            info!(logger, "Updated 1 package");
        }
        _ => {
            info!(logger, "Updated {} package", num_updated);
        }
    }
    Ok(())
}

pub fn install_future<'client,'a: 'client,  C, I: 'a,>(
    config: &'a Config,
    pdscs: I,
    client: &'client Client<C, Body>,
    logger: &'a Logger,
    spin: bool,
) -> impl Future<Item = Vec<PathBuf>, Error = Error> + 'client
    where C: Connect,
          I: IntoIterator<Item = &'a Package>,
{
    let packs = download_pack_stream(config, iter_ok(pdscs), client, logger, spin);
    packs.filter_map(id).collect()
}

// This will "trick" the borrow checker into thinking that the lifetimes for
// client and core are at least as big as the lifetime for pdscs, which they actually are
fn install_inner<'client, 'a: 'client, C, I: 'a>(
    config: &'a Config,
    pdsc_list: I,
    core: &mut Core,
    client: &'client Client<C, Body>,
    logger: &'a Logger,
    spin: bool,
) -> Result<Vec<PathBuf>, Error>
    where
    C: Connect,
    I: IntoIterator<Item = &'a Package>,
{
    core.run(install_future(config, pdsc_list, client, logger, spin))
}

/// Flatten a list of Vidx Urls into a list of updated CMSIS packs
pub fn install<'a, I: 'a>(
    config: &'a Config,
    pdsc_list: I,
    logger: &'a Logger,
    spin: bool,
) -> Result<Vec<PathBuf>, Error>
    where
    I: IntoIterator<Item = &'a Package>,
{
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let client = Client::configure()
        .keep_alive(true)
        .connector(HttpsConnector::new(4, &handle).unwrap())
        .build(&handle);
    install_inner(config, pdsc_list, &mut core, &client, logger, spin)
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
    logger: &Logger
) -> Result<(), Error> {
    let pdsc_list: Vec<_> = args.values_of("PDSC")
        .unwrap()
        .filter_map(|input| Package::from_path(Path::new(input), logger).ok())
        .collect();
    let updated = install(conf, pdsc_list.iter(), logger, true)?;
    let num_updated = updated.iter().map(|_| 1).sum::<u32>();
    match num_updated {
        0 => {
            info!(logger, "Already up to date");
        }
        1 => {
            info!(logger, "Updated 1 package");
        }
        _ => {
            info!(logger, "Updated {} package", num_updated);
        }
    }
    Ok(())
}
