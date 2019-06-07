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

use hyper::Client;
use hyper_rustls::HttpsConnector;
use tokio_core::reactor::Core;
use std::path::PathBuf;
use slog::Logger;
use failure::Error;
use futures::stream::iter_ok;
use futures::Stream;

use pdsc::Package;

mod redirect;
mod vidx;
mod download;

use vidx::{download_vidx_list, flatmap_pdscs};
pub use download::{DownloadConfig, DownloadContext, DownloadProgress};

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
    let dl_cntx = DownloadContext::new(config, progress, &client, logger);
    let fut = {
        let parsed_vidx = download_vidx_list(vidx_list, &client, logger);
        let pdsc_list = parsed_vidx
            .filter_map(|vidx| vidx.map(|v| flatmap_pdscs(v, &client, logger)))
            .flatten();
        dl_cntx.download_stream(pdsc_list).collect()
    };
    core.run(fut)
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
    let dl_cntx = DownloadContext::new(config, progress, &client, logger);
    let fut = {
        dl_cntx.download_stream(iter_ok(pdsc_list)).collect()
    };
    core.run(fut)
}
