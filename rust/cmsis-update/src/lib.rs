use failure::Error;
use std::path::PathBuf;

use futures::stream::iter_ok;
use futures::Stream;
use slog::Logger;
use tokio_core::reactor::Core;

use pdsc::Package;

mod download;

use crate::download::DownloadContext;
pub use crate::download::{DownloadConfig, DownloadProgress};

type Result<T> = std::result::Result<T, Error>;

/// Flatten a list of Vidx Urls into a list of updated CMSIS packs
pub fn update<I, P, D>(
    config: &D,
    vidx_list: I,
    logger: &Logger,
    progress: P,
) -> Result<Vec<PathBuf>>
where
    I: IntoIterator<Item = String>,
    P: DownloadProgress,
    D: DownloadConfig,
{
    let mut core = Core::new().unwrap();
    let dl_cntx = DownloadContext::new(config, progress, logger)?;
    let fut = {
        let parsed_vidx = dl_cntx.download_vidx_list(vidx_list);
        let pdsc_list = parsed_vidx
            .filter_map(|vidx| vidx.map(|v| dl_cntx.flatmap_pdscs(v)))
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
    progress: P,
) -> Result<Vec<PathBuf>>
where
    I: IntoIterator<Item = &'a Package>,
    P: DownloadProgress + 'a,
    D: DownloadConfig,
{
    let mut core = Core::new().unwrap();
    let dl_cntx = DownloadContext::new(config, progress, logger)?;
    let fut = dl_cntx.download_stream(iter_ok(pdsc_list)).collect();
    core.run(fut)
}
