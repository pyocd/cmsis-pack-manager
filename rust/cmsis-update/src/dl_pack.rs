use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use failure::Error;
use futures::Stream;
use futures::prelude::*;
use hyper::{Body, Client, Uri};
use hyper::client::Connect;
use slog::Logger;

use pdsc::Package;
use pack_index::config::Config;

use redirect::ClientRedirExt;

fn make_uri(
    &Package{
        ref name,
        ref vendor,
        ref url,
        ref releases,
        ..
    }: &Package
) -> Result<Uri, Error> {
    let version: &str = releases.latest_release().version.as_ref();
    let uri = if url.ends_with('/') {
        format!("{}{}.{}.{}.pack", url, vendor, name, version)
    } else {
        format!("{}/{}.{}.{}.pdsc", url, vendor, name, version)
    }.parse()?;
    Ok(uri)
}

fn make_fd(
    config: &Config,
    &Package{
        ref name,
        ref vendor,
        ref releases,
        ..
    }: &Package,
) -> PathBuf {
    let version: &str = releases.latest_release().version.as_ref();
    let mut filename = config.pack_store.clone();
    filename.push(Path::new(vendor));
    filename.push(Path::new(name));
    filename.push(format!("{}.pack", version));
    filename
}

fn download_pack<'a, C: Connect>(
    config: &'a Config,
    pdsc: Package,
    client: &'a Client<C, Body>,
    logger: &'a Logger,
) -> impl Future<Item = Option<PathBuf>, Error = Error> + 'a {
    async_block!{
        let filename = make_fd(config, &pdsc);
        if filename.exists() {
            return Ok(None);
        }
        create_dir_all(filename.parent().unwrap())?;
        let uri = make_uri(&pdsc)?;
        let mut fd = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&filename)?;
        let response = await!(client.redirectable(uri, logger))?;
        #[async]
        for bytes in response.body() {
            fd.write_all(bytes.as_ref())?;
        }
        Ok(Some(filename))
    }
}

pub(crate) fn download_pack_stream<'a, F, C>(
    config: &'a Config,
    stream: F,
    client: &'a Client<C, Body>,
    logger: &'a Logger,
) -> impl Stream<Item = Option<PathBuf>, Error = Error> + 'a
    where
    F: Stream<Item = Package, Error = Error> + 'a,
    C: Connect,
{
    stream
        .map(move |pdsc| download_pack(config, pdsc, client, logger))
        .buffer_unordered(32)
}
