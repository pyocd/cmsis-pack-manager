use std::path::PathBuf;

use failure::Error;
use futures::prelude::*;
use hyper::{Body, Client, Uri};
use hyper::client::Connect;
use slog::Logger;

use pack_index::{PdscRef};
use pack_index::config::Config;

use download::{IntoDownload, DownloadProgress, download_stream};
use vidx::{download_vidx_list, flatmap_pdscs};

impl IntoDownload for PdscRef {
    fn into_uri(&self, _: &Config) -> Result<Uri, Error> {
        let &PdscRef {ref url, ref vendor, ref name, ..} = self;
        let uri = if url.ends_with('/') {
            format!("{}{}.{}.pdsc", url, vendor, name)
        } else {
            format!("{}/{}.{}.pdsc", url, vendor, name)
        }.parse()?;
        Ok(uri)
    }

    fn into_fd(&self, config: &Config) -> PathBuf {
        let &PdscRef {ref vendor, ref name, ref version, ..} = self;
        let mut filename = config.pack_store.clone();
        let pdscname = format!("{}.{}.{}.pdsc", vendor, name, version);
        filename.push(pdscname);
        filename
    }
}

/// Create a future of the update command.
pub fn update_future<'a, C, I, P>(
    config: &'a Config,
    vidx_list: I,
    client: &'a Client<C, Body>,
    logger: &'a Logger,
    progress: P
) -> impl Future<Item = Vec<PathBuf>, Error = Error> + 'a
    where C: Connect,
          I: IntoIterator<Item = String> + 'a,
          P: DownloadProgress + 'a,
{
    let parsed_vidx = download_vidx_list(vidx_list, client, logger);
    let pdsc_list = parsed_vidx
        .filter_map(move |vidx| match vidx {
            Ok(v) => Some(flatmap_pdscs(v, client, logger)),
            Err(_) => None,
        })
        .flatten();
    download_stream(config, pdsc_list, client, logger, progress).collect()
}
