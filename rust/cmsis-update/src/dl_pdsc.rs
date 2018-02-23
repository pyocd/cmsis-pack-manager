use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use failure::Error;
use futures::Stream;
use futures::prelude::*;
use hyper::{Body, Client, Uri};
use hyper::client::Connect;
use indicatif::{ProgressBar, ProgressStyle};
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
    indicator: Option<Arc<ProgressBar>>
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
            if let Some(ref spinner) = indicator {
                spinner.inc(bytes.len() as u64);
            }
        }
        Ok(Some(filename))
    }
}

pub(crate) fn download_pdsc_stream<'a, F, C>(
    config: &'a Config,
    stream: F,
    client: &'a Client<C, Body>,
    logger: &'a Logger,
    spin: bool,
) -> impl Stream<Item = Option<PathBuf>, Error = Error> + 'a
where
    F: Stream<Item = PdscRef, Error = Error> + 'a,
    C: Connect,
{
    async_stream_block!(
        if spin {
            let spinner = Arc::new(ProgressBar::new_spinner());
            spinner.set_style(ProgressStyle::default_spinner()
                              .template("[{elapsed_precise}] Downloading Pack Descriptions {spinner} {bytes}"));
            #[async]
            for pdsc_ref in stream {
                stream_yield!(download_pdsc(config, pdsc_ref, client,
                                            logger, Some(spinner.clone())))
            }
        } else {
            #[async]
            for pdsc_ref in stream {
                stream_yield!(download_pdsc(config, pdsc_ref, client,
                                            logger, None))
            }
        }
        Ok(())
    ).buffer_unordered(32)
}
