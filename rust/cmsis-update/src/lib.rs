extern crate futures;
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

use hyper::{Body, Client};
use hyper::client::Connect;
use hyper_rustls::HttpsConnector;
use tokio_core::reactor::Core;
use std::path::PathBuf;
use slog::Logger;
use failure::Error;

use pdsc::Package;

mod redirect;
mod vidx;
mod download;
mod dl_pdsc;
mod dl_pack;

use dl_pdsc::{update_future};
use dl_pack::{install_future};
pub use download::DownloadProgress;
pub use download::DownloadConfig;


// This will "trick" the borrow checker into thinking that the lifetimes for
// client and core are at least as big as the lifetime for pdscs, which they actually are
fn update_inner<'a, C, I, P, D>(
    config: &'a D,
    vidx_list: I,
    core: &'a mut Core,
    client: &'a Client<C, Body>,
    logger: &'a Logger,
    progress: P,
) -> Result<Vec<PathBuf>, Error>
where
    C: Connect,
    I: IntoIterator<Item = String>,
    P: DownloadProgress + 'a,
    D: DownloadConfig,
{
    core.run(update_future(config, vidx_list, client, logger, progress))
}

/// Flatten a list of Vidx Urls into a list of updated CMSIS packs
pub fn update<I, P, D>(config: &D, vidx_list: I, logger: &Logger, progress: P) -> Result<Vec<PathBuf>, Error>
where
    I: IntoIterator<Item = String>,
    P: DownloadProgress,
    D: DownloadConfig,
{
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let client: Client<HttpsConnector, _> = Client::configure()
        .keep_alive(true)
        .connector(HttpsConnector::new(4, &handle))
        .build(&handle);
    update_inner(config, vidx_list, &mut core, &client, logger, progress)
}

// This will "trick" the borrow checker into thinking that the lifetimes for
// client and core are at least as big as the lifetime for pdscs, which they actually are
fn install_inner<'client, 'a: 'client, C, I: 'a, P: 'client, D>(
    config: &'a D,
    pdsc_list: I,
    core: &mut Core,
    client: &'client Client<C, Body>,
    logger: &'a Logger,
    progress: P
) -> Result<Vec<PathBuf>, Error>
    where
    C: Connect,
    I: IntoIterator<Item = &'a Package>,
    P: DownloadProgress + 'a,
    D: DownloadConfig,
{
    core.run(install_future(config, pdsc_list, client, logger, progress))
}

/// Flatten a list of Vidx Urls into a list of updated CMSIS packs
pub fn install<'a, I: 'a, P, D>(
    config: &'a D,
    pdsc_list: I,
    logger: &'a Logger,
    progress: P
) -> Result<Vec<PathBuf>, Error>
    where
    I: IntoIterator<Item = &'a Package>,
    P: DownloadProgress + 'a,
    D: DownloadConfig,
{
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let client: Client<HttpsConnector, _> = Client::configure()
        .keep_alive(true)
        .connector(HttpsConnector::new(4, &handle))
        .build(&handle);
    install_inner(config, pdsc_list, &mut core, &client, logger, progress)
}
