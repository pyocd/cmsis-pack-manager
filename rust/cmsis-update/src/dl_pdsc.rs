use std::path::PathBuf;

use failure::Error;
use futures::prelude::*;
use hyper::{Body, Client, Uri};
use hyper::client::Connect;
use slog::Logger;

use pack_index::{PdscRef};

use download::{IntoDownload, DownloadProgress, DownloadConfig, download_stream};
use vidx::{download_vidx_list, flatmap_pdscs};

impl IntoDownload for PdscRef {
    fn into_uri(&self) -> Result<Uri, Error> {
        let &PdscRef {ref url, ref vendor, ref name, ..} = self;
        let uri = if url.ends_with('/') {
            format!("{}{}.{}.pdsc", url, vendor, name)
        } else {
            format!("{}/{}.{}.pdsc", url, vendor, name)
        }.parse()?;
        Ok(uri)
    }

    fn into_fd<D: DownloadConfig>(&self, config: &D) -> PathBuf {
        let &PdscRef {ref vendor, ref name, ref version, ..} = self;
        let mut filename = config.pack_store();
        let pdscname = format!("{}.{}.{}.pdsc", vendor, name, version);
        filename.push(pdscname);
        filename
    }
}

/// Create a future of the update command.
pub fn update_future<'a, C, I, P, D>(
    config: &'a D,
    vidx_list: I,
    client: &'a Client<C, Body>,
    logger: &'a Logger,
    progress: P
) -> impl Future<Item = Vec<PathBuf>, Error = Error> + 'a
    where C: Connect,
          I: IntoIterator<Item = String> + 'a,
          P: DownloadProgress + 'a,
          D: DownloadConfig,
{
    let parsed_vidx = download_vidx_list(vidx_list, client, logger);
    let pdsc_list = parsed_vidx
        .filter_map(move |vidx| vidx.map(|v| flatmap_pdscs(v, client, logger)))
        .flatten();
    download_stream(config, pdsc_list, client, logger, progress).collect()
}
