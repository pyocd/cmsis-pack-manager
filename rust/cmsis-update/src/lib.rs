#![feature(generators, libc, proc_macro_hygiene)]

extern crate futures_await as futures;
extern crate tokio_core;
extern crate hyper;
extern crate hyper_rustls;
extern crate minidom;
extern crate failure;

#[macro_use]
extern crate slog;

extern crate utils;
extern crate pack_index;
extern crate pdsc;
extern crate pbr;

use std::sync::Mutex;

use hyper::{Body, Client};
use hyper::client::Connect;
use hyper_rustls::HttpsConnector;
use tokio_core::reactor::Core;
use std::path::PathBuf;
use slog::Logger;
use failure::Error;
use pbr::ProgressBar;

use pack_index::config::Config;
use pdsc::Package;

pub mod upgrade;
mod redirect;
mod vidx;
mod download;
mod dl_pdsc;
mod dl_pack;

use dl_pdsc::{update_future};
use dl_pack::{install_future};
use download::DownloadProgress;

// This will "trick" the borrow checker into thinking that the lifetimes for
// client and core are at least as big as the lifetime for pdscs, which they actually are
fn update_inner<C, I, P>(
    config: &Config,
    vidx_list: I,
    core: &mut Core,
    client: &Client<C, Body>,
    logger: &Logger,
    progress: P,
) -> Result<Vec<PathBuf>, Error>
where
    C: Connect,
    I: IntoIterator<Item = String>,
    P: DownloadProgress,
{
    core.run(update_future(config, vidx_list, client, logger, progress))
}

/// Flatten a list of Vidx Urls into a list of updated CMSIS packs
pub fn update<I>(config: &Config, vidx_list: I, logger: &Logger) -> Result<Vec<PathBuf>, Error>
where
    I: IntoIterator<Item = String>,
{
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let client: Client<HttpsConnector, _> = Client::configure()
        .keep_alive(true)
        .connector(HttpsConnector::new(4, &handle))
        .build(&handle);
    let mut progress = ProgressBar::new(363);
    progress.show_speed = false;
    progress.show_time_left = false;
    progress.format("[#> ]");
    progress.message("Downloading Descriptions ");
    let progress = Mutex::new(progress);
    update_inner(config, vidx_list, &mut core, &client, logger, &progress)
}

// This will "trick" the borrow checker into thinking that the lifetimes for
// client and core are at least as big as the lifetime for pdscs, which they actually are
fn install_inner<'client, 'a: 'client, C, I: 'a, P: 'client>(
    config: &'a Config,
    pdsc_list: I,
    core: &mut Core,
    client: &'client Client<C, Body>,
    logger: &'a Logger,
    progress: P
) -> Result<Vec<PathBuf>, Error>
    where
    C: Connect,
    I: IntoIterator<Item = &'a Package>,
    P: DownloadProgress
{
    core.run(install_future(config, pdsc_list, client, logger, progress))
}

/// Flatten a list of Vidx Urls into a list of updated CMSIS packs
pub fn install<'a, I: 'a>(
    config: &'a Config,
    pdsc_list: I,
    logger: &'a Logger
) -> Result<Vec<PathBuf>, Error>
    where
    I: IntoIterator<Item = &'a Package>,
{
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let client: Client<HttpsConnector, _> = Client::configure()
        .keep_alive(true)
        .connector(HttpsConnector::new(4, &handle))
        .build(&handle);
    let mut progress = ProgressBar::new(363);
    progress.show_speed = false;
    progress.show_time_left = false;
    progress.format("[#> ]");
    progress.message("Downloading Packs ");
    let progress = Mutex::new(progress);
    install_inner(config, pdsc_list, &mut core, &client, logger, &progress)
}
