use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use failure::Error;
use futures::Stream;
use futures::prelude::*;
use hyper::{Body, Client, Uri};
use hyper::client::Connect;
use indicatif::{ProgressBar, ProgressStyle};
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

fn download_pack<'b, 'a: 'b,  C: Connect>(
    config: &'a Config,
    pdsc: &'a Package,
    client: &'b Client<C, Body>,
    logger: &'a Logger,
    indicator: Option<Arc<ProgressBar>>
) -> impl Future<Item = Option<PathBuf>, Error = Error> + 'b {
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
            if let Some(ref spinner) = indicator {
                spinner.inc(bytes.len() as u64);
            }
        }
        Ok(Some(filename))
    }
}

pub(crate) fn download_pack_stream<'a, 'b, F, C>(
    config: &'a Config,
    stream: F,
    client: &'b Client<C, Body>,
    logger: &'a Logger,
    spin: bool,
) -> impl Stream<Item = Option<PathBuf>, Error = Error> + 'b
    where
    F: Stream<Item = &'a Package, Error = Error> + 'b,
    C: Connect,
    'a: 'b
{
    async_stream_block!(
        if spin {
            let spinner = Arc::new(ProgressBar::new_spinner());
            spinner.set_style(ProgressStyle::default_spinner()
                              .template("[{elapsed_precise}] Downloading Packs {spinner} {bytes}"));
            #[async]
            for pdsc_ref in stream {
                stream_yield!(download_pack(config, pdsc_ref, client,
                                            logger, Some(spinner.clone())))
            }
        } else {
            #[async]
            for pdsc_ref in stream {
                stream_yield!(download_pack(config, pdsc_ref, client,
                                            logger, None))
            }
        }
        Ok(())
    ).buffer_unordered(32)
}
