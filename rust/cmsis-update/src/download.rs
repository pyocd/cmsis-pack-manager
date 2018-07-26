use std::fs::{create_dir_all, rename, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use failure::Error;
use futures::Stream;
use futures::prelude::{await, async_block, async_stream_block, stream_yield, Future};
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

pub trait DownloadProgress: Sync {
    fn size(&self, files: usize);
    fn progress(&self, bytes: usize);
    fn complete(&self);
    fn for_file(&self, file: &str) -> Self;
}

impl DownloadProgress for () {
    fn size(&self, _: usize) {}
    fn progress(&self, _: usize) {}
    fn complete(&self) {}
    fn for_file(&self, _: &str) -> Self {
        ()
    }
}

impl<'a, W: Write + Send + 'a> DownloadProgress for &'a Mutex<ProgressBar<W>> {
    fn size(&self, files: usize) {
        if let Ok(mut inner) = self.lock() {
            inner.total = files as u64;
            inner.show_speed = false;
            inner.show_bar = true;
        }
    }
    fn progress(&self, _: usize) {}
    fn complete(&self) {
        if let Ok(mut inner) = self.lock() {
            inner.inc();
        }
    }
    fn for_file(&self, _: &str) -> Self {
        self.clone()
    }
}

fn download_file<'b,  C: Connect, P: DownloadProgress + 'b>(
    source: Uri,
    dest: PathBuf,
    client: &'b Client<C, Body>,
    logger: &'b Logger,
    spinner: P
) -> impl Future<Item = PathBuf, Error = Error> + 'b {
    async_block!{
        let response = await!(client.redirectable(source, logger))?;
        let temp = dest.with_extension("part");
        let mut fd = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&temp)?;
        #[async]
        for bytes in response.body() {
            fd.write_all(bytes.as_ref())?;
            spinner.progress(bytes.len());
        }
        rename(&temp, &dest)?;
        spinner.complete();
        Ok(dest)
    }
}

pub(crate) fn download_stream<'b, 'a: 'b, F, C, P: 'b, DL: 'a>(
    config: &'a Config,
    stream: F,
    client: &'b Client<C, Body>,
    logger: &'a Logger,
    progress: P
) -> Box<Stream<Item = PathBuf, Error = Error> + 'b>
    where F: Stream<Item = DL, Error = Error> + 'b,
          C: Connect,
          DL: IntoDownload,
          P: DownloadProgress
{
    Box::new(
        async_stream_block!(
            let to_dl = await!(stream.collect())?;
            let len = to_dl.iter().filter_map(|dl| should_download(config, dl)).count();
            progress.size(len);
            for from in to_dl {
                if let Some(dest) = should_download(config, &from) {
                    let source = from.into_uri(config)?;
                    let new_prog = progress.for_file(&dest.to_string_lossy());
                    stream_yield!(download_file(source, dest, client, logger, new_prog))
                }
            }
            Ok(())
        ).buffer_unordered(32)
    )
}
