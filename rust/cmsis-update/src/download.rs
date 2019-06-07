use std::fs::{create_dir_all, rename, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use failure::Error;
use futures::Stream;
use futures::prelude::Future;
use futures::future::{ok, result};
use futures::stream::iter_ok;
use hyper::{Body, Client, Uri};
use hyper::client::Connect;
use slog::Logger;
use std::sync::Arc;

use redirect::ClientRedirExt;

pub trait DownloadConfig {
    fn pack_store(&self) -> PathBuf;
}

pub(crate) trait IntoDownload {
    fn into_uri(&self) -> Result<Uri, Error>;
    fn into_fd<D: DownloadConfig>(&self, &D) -> PathBuf;
}

pub trait DownloadProgress: Send {
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

fn download_file<'b,  C: Connect, P: DownloadProgress + 'b>(
    source: Uri,
    dest: PathBuf,
    client: &'b Client<C, Body>,
    logger: &'b Logger,
    spinner: Arc<P>
) -> Box<Future<Item=(), Error=Error> + 'b> {
    if !dest.exists() {
        dest.parent().map(create_dir_all);
        Box::new(client.redirectable(source, logger)
             .from_err()
             .and_then(move |res| {
                let temp = dest.with_extension("part");
                let fdf = result(OpenOptions::new()
                    .write(true)
                    .create(true)
                    .open(&temp));
                fdf.from_err().and_then(move |mut fd| {
                    res.body().for_each(move |bytes| {
                        spinner.progress(bytes.len());
                        fd.write_all(bytes.as_ref())?;
                        Ok(())
                    }).then(move |_| {
                        rename(&temp, &dest)?;
                        Ok(())
                    })
                })
            })
        )
    } else {
        Box::new(ok(()))
    }
}

pub(crate) fn download_stream<'b, F, C, P: 'b, DL: 'b, D>(
    config: &'b D,
    stream: F,
    client: &'b Client<C, Body>,
    logger: &'b Logger,
    progress: P
) -> Box<Stream<Item = PathBuf, Error = Error> + 'b>
    where F: Stream<Item = DL, Error = Error> + 'b,
          C: Connect,
          DL: IntoDownload,
          P: DownloadProgress,
          D: DownloadConfig,
{
    let streaming_pathbuffs = 
        stream.collect().map(move |to_dl|{
            let len = to_dl.len();
            progress.size(len);
            iter_ok(to_dl).map(move |from| {
                let dest = from.into_fd(config);
                let source = from.into_uri();
                let new_prog = Arc::new(progress.for_file(&dest.to_string_lossy()));
                result(source).and_then(move |source| download_file(
                    source.clone(), dest.clone(), client, logger, new_prog.clone()
                    ).then(move |res| {
                        new_prog.complete();
                        match res {
                            Ok(_) => Ok(Some(dest)),
                            Err(e) => {
                                slog_error!(logger, "download of {:?} failed: {}", source, e);
                                Ok(None)
                            }
                        }
                    }))
            })
        }).flatten_stream();
    Box::new(streaming_pathbuffs.buffer_unordered(32).filter_map(|x| x))
}
