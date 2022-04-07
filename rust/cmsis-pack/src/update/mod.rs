use anyhow::Error;
use std::path::PathBuf;
use tokio::runtime;

use crate::pdsc::Package;

mod download;

use crate::update::download::DownloadContext;
pub use crate::update::download::{DownloadConfig, DownloadProgress};

type Result<T> = std::result::Result<T, Error>;

/// Flatten a list of Vidx Urls into a list of updated CMSIS packs
pub fn update<I, P, D>(config: &D, vidx_list: I, progress: P) -> Result<Vec<PathBuf>>
where
    I: IntoIterator<Item = String>,
    P: DownloadProgress,
    D: DownloadConfig,
{
    let rt = runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let dl_cntx = DownloadContext::new(config, progress)?;
    rt.block_on(dl_cntx.update_vidx(vidx_list))
}

/// Flatten a list of Vidx Urls into a list of updated CMSIS packs
pub fn install<'a, I: 'a, P, D>(config: &'a D, pdsc_list: I, progress: P) -> Result<Vec<PathBuf>>
where
    I: IntoIterator<Item = &'a Package>,
    P: DownloadProgress + 'a,
    D: DownloadConfig,
{
    let rt = runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let dl_cntx = DownloadContext::new(config, progress)?;
    rt.block_on(async { Ok(dl_cntx.download_iterator(pdsc_list).await) })
}
