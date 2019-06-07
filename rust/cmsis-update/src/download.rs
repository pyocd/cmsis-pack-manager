use std::fs::{create_dir_all, rename, OpenOptions};
use std::io::Write;
use std::path::{PathBuf, Path};

use failure::Error;
use futures::Stream;
use futures::prelude::Future;
use futures::future::{ok, result};
use futures::stream::iter_ok;
use hyper::{Body, Client, Uri};
use hyper::client::Connect;
use slog::Logger;

use pdsc::Package;
use pack_index::PdscRef;

use redirect::ClientRedirExt;

pub trait DownloadConfig {
    fn pack_store(&self) -> PathBuf;
}

pub trait IntoDownload {
    fn into_uri(&self) -> Result<Uri, Error>;
    fn into_fd<D: DownloadConfig>(&self, &D) -> PathBuf;
}

impl IntoDownload for PdscRef {
    fn into_uri(&self) -> Result<Uri, Error> {
        let &PdscRef {ref url, ref vendor, ref name, ..} = self;
        let uri = if url.ends_with('/') {
            format!("{}{}.{}.pdsc", url, vendor, name)
        } else {
            format!("{}/{}.{}.pdsc", url, vendor, name)
        }.parse()?;
        Ok(uri)
    }

    fn into_fd<D: DownloadConfig>(&self, config: &D) -> PathBuf {
        let &PdscRef {ref vendor, ref name, ref version, ..} = self;
        let mut filename = config.pack_store();
        let pdscname = format!("{}.{}.{}.pdsc", vendor, name, version);
        filename.push(pdscname);
        filename
    }
}

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

pub struct DownloadContext<'a, Conf, Prog, Con>
where Conf: DownloadConfig,
      Prog: DownloadProgress + 'a,
      Con: Connect,
{
    config: &'a Conf,
    prog:  Prog,
    client: &'a Client<Con, Body>,
    log: &'a Logger,
}

impl<'a, Conf, Prog, Con> DownloadContext<'a, Conf, Prog, Con>
where Conf: DownloadConfig,
      Prog: DownloadProgress + 'a,
      Con: Connect,
{
    pub fn new(config: &'a Conf, prog: Prog, client: &'a Client<Con, Body>, log: &'a Logger) -> Self {
        DownloadContext {
            config,
            prog,
            client,
            log
        }
    }

    fn download_file(
        &'a self,
        source: Uri,
        dest: PathBuf,
    ) -> Box<Future<Item=(), Error=Error> + 'a> {
        if !dest.exists() {
            dest.parent().map(create_dir_all);
            Box::new(self.client.redirectable(source, self.log)
                 .from_err()
                 .and_then(move |res| {
                    let temp = dest.with_extension("part");
                    let fdf = result(OpenOptions::new()
                        .write(true)
                        .create(true)
                        .open(&temp));
                    fdf.from_err().and_then(move |mut fd| {
                        res.body().for_each(move |bytes| {
                            self.prog.progress(bytes.len());
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

    pub fn download_stream<F, DL>(&'a self, stream: F) -> Box<Stream<Item = PathBuf, Error = Error> + 'a>
    where F: Stream<Item = DL, Error = Error> + 'a,
          DL: IntoDownload + 'a,
    {
        let streaming_pathbuffs = 
            stream.collect().map(move |to_dl|{
                let len = to_dl.len();
                self.prog.size(len);
                iter_ok(to_dl).map(move |from| {
                    let dest = from.into_fd(self.config);
                    let source = from.into_uri();
                    result(source).and_then(move |source| self.download_file(
                        source.clone(), dest.clone()
                        ).then(move |res| {
                            self.prog.complete();
                            match res {
                                Ok(_) => Ok(Some(dest)),
                                Err(e) => {
                                    slog_error!(self.log, "download of {:?} failed: {}", source, e);
                                    Ok(None)
                                }
                            }
                        }))
                })
            }).flatten_stream();
        Box::new(streaming_pathbuffs.buffer_unordered(32).filter_map(|x| x))
    }

}
