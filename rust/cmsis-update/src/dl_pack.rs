use std::path::{Path, PathBuf};

use failure::Error;
use futures::stream::iter_ok;
use futures::prelude::*;
use hyper::{Body, Client, Uri};
use hyper::client::Connect;
use slog::Logger;

use pdsc::Package;

use download::{IntoDownload, DownloadProgress, DownloadConfig, download_stream};

impl<'a> IntoDownload for &'a Package {
    fn into_uri(&self) -> Result<Uri, Error> {
        let &Package{ref name, ref vendor, ref url, ref releases, ..} = *self;
        let version: &str = releases.latest_release().version.as_ref();
        let uri = if url.ends_with('/') {
            format!("{}{}.{}.{}.pack", url, vendor, name, version)
        } else {
            format!("{}/{}.{}.{}.pack", url, vendor, name, version)
        }.parse()?;
        Ok(uri)
    }

    fn into_fd<D: DownloadConfig>(&self, config: &D) -> PathBuf {
        let &Package{ref name, ref vendor, ref releases, ..} = *self;
        let version: &str = releases.latest_release().version.as_ref();
        let mut filename = config.pack_store();
        filename.push(Path::new(vendor));
        filename.push(Path::new(name));
        filename.push(format!("{}.pack", version));
        filename
    }
}


pub fn install_future<'client,'a: 'client,  C, I, P, D>(
    config: &'a D,
    pdscs: I,
    client: &'client Client<C, Body>,
    logger: &'a Logger,
    progress: P,
) -> impl Future<Item = Vec<PathBuf>, Error = Error> + 'client
    where C: Connect,
          I: IntoIterator<Item = &'a Package> + 'a,
          P: DownloadProgress + 'client,
          D: DownloadConfig + 'a
{
    download_stream(config, iter_ok(pdscs), client, logger, progress).collect()
}
