use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use failure::Error;
use futures::Stream;
use futures::prelude::*;
use hyper::{Body, Client, Uri};
use hyper::client::Connect;
use slog::Logger;

use pack_index::{PdscRef};
use pack_index::config::Config;

use redirect::ClientRedirExt;

fn make_uri(
    &PdscRef {
        ref url,
        ref vendor,
        ref name,
        ..
    }: &PdscRef,
) -> Result<Uri, Error> {
    let uri = if url.ends_with('/') {
        format!("{}{}.{}.pdsc", url, vendor, name)
    } else {
        format!("{}/{}.{}.pdsc", url, vendor, name)
    }.parse()?;
    Ok(uri)
}

fn make_fd(
    config: &Config,
    &PdscRef {
        ref vendor,
        ref name,
        ref version,
        ..
    }: &PdscRef,
) -> PathBuf {
    let mut filename = config.pack_store.clone();
    let pdscname = format!("{}.{}.{}.pdsc", vendor, name, version);
    filename.push(pdscname);
    filename
}

fn download_pdsc<'a, C: Connect>(
    config: &'a Config,
    pdsc_ref: PdscRef,
    client: &'a Client<C, Body>,
    logger: &'a Logger,
) -> impl Future<Item = Option<PathBuf>, Error = Error> + 'a {
    async_block!{
        let filename = make_fd(config, &pdsc_ref);
        if filename.exists() {
            return Ok(None);
        }
        let uri = make_uri(&pdsc_ref)?;
        let PdscRef{vendor, name, version, ..} = pdsc_ref;
        debug!(logger, "Updating package {}::{} to version {}", vendor, name, version);
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

pub(crate) fn download_pdsc_stream<'a, F, C>(
    config: &'a Config,
    stream: F,
    client: &'a Client<C, Body>,
    logger: &'a Logger,
) -> impl Stream<Item = Option<PathBuf>, Error = Error> + 'a
where
    F: Stream<Item = PdscRef, Error = Error> + 'a,
    C: Connect,
{
    stream
        .map(move |pdsc_ref| download_pdsc(config, pdsc_ref, client, logger))
        .buffer_unordered(32)
}
