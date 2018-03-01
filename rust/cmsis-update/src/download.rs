use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use failure::Error;
use futures::Stream;
use futures::prelude::*;
use hyper::{Body, Client, Uri};
use hyper::client::Connect;
use slog::Logger;

use pack_index::config::Config;

use redirect::ClientRedirExt;

pub(crate) trait IntoDownload {
    fn into_uri(&self, &Config) -> Result<Uri, Error>;
    fn into_fd(&self, &Config) -> PathBuf;
}

fn download_file<'b, 'a: 'b,  C: Connect, DL: IntoDownload + 'b>(
    config: &'a Config,
    from: DL,
    client: &'b Client<C, Body>,
    logger: &'a Logger,
) -> impl Future<Item = Option<PathBuf>, Error = Error> + 'b {
    async_block!{
        let dest = from.into_fd(config);
        if dest.exists() {
            return Ok(None);
        }
        dest.parent().map(create_dir_all);
        let mut fd = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&dest)?;
        let source = from.into_uri(config)?;
        let response = await!(client.redirectable(source, logger))?;
        #[async]
        for bytes in response.body() {
            fd.write_all(bytes.as_ref())?;
        }
        Ok(Some(dest))
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
        #[async]
        for from in stream {
            stream_yield!(download_file(config, from, client, logger))
        }
        Ok(())
    ).buffer_unordered(32).filter_map(id)
}

fn id<T>(slf: T) -> T {
    slf
}
