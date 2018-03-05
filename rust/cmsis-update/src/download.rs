use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use failure::Error;
use futures::Stream;
use futures::prelude::*;
use hyper::{Body, Client, Uri};
use hyper::client::Connect;
use slog::Logger;
use pbr::ProgressBar;

use pack_index::config::Config;

use redirect::ClientRedirExt;

pub(crate) trait IntoDownload {
    fn into_uri(&self, &Config) -> Result<Uri, Error>;
    fn into_fd(&self, &Config) -> PathBuf;
}

fn should_download<'a, DL: IntoDownload>(config: &Config, from: &'a DL) -> Option<PathBuf> {
    let dest = from.into_fd(config);
    if dest.exists() {
        None
    } else {
        dest.parent().map(create_dir_all);
        Some(dest)
    }
}

fn download_file<'b, 'a: 'b,  C: Connect, W: Write + 'b>(
    source: Uri,
    dest: PathBuf,
    client: &'b Client<C, Body>,
    logger: &'a Logger,
    spinner: Option<Arc<Mutex<ProgressBar<W>>>>
) -> impl Future<Item = PathBuf, Error = Error> + 'b {
    async_block!{
        let response = await!(client.redirectable(source, logger))?;
        let mut fd = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&dest)?;
        #[async]
        for bytes in response.body() {
            fd.write_all(bytes.as_ref())?;
        }
        if let Some(ref spin) = spinner {
            if let Ok(mut inner) = spin.lock() {
                inner.inc();
            }
        }
        Ok(dest)
    }
}

pub(crate) fn download_stream<'b, 'a: 'b, F, C, DL: 'a>(
    config: &'a Config,
    stream: F,
    client: &'b Client<C, Body>,
    logger: &'a Logger,
) -> impl Stream<Item = PathBuf, Error = Error> + 'b
    where F: Stream<Item = DL, Error = Error> + 'b,
          C: Connect,
          DL: IntoDownload,
{
    async_stream_block!(
        let to_dl = await!(stream.collect())?;
        let len = to_dl.iter().filter_map(|dl| should_download(config, dl)).count();
        let mut pb = ProgressBar::new(len as u64);
        pb.tick_format("/\\-");
        pb.set_max_refresh_rate(Some(Duration::from_millis(100)));
        pb.message("Downloading Pack Descriptions ");
        pb.show_bar = true;
        pb.show_tick = false;
        pb.show_speed = false;
        pb.show_percent = true;
        pb.show_counter = true;
        pb.show_time_left = false;
        pb.tick();
        let pb = Arc::new(Mutex::new(pb));
        for from in to_dl {
            if let Some(dest) = should_download(config, &from) {
                let source = from.into_uri(config)?;
                stream_yield!(download_file(source, dest, client, logger, Some(pb.clone())))
            }
        }
        Ok(())
    ).buffer_unordered(32)
}
