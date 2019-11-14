use std::borrow::Borrow;
use std::fs::{create_dir_all, rename, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use failure::Error;
use futures::future::{ok, result};
use futures::prelude::Future;
use futures::stream::{futures_unordered, iter_ok};
use futures::Stream;
use reqwest::r#async::{Chunk, Client, ClientBuilder, Response};
use reqwest::{RedirectPolicy, Url, UrlError};

use crate::pack_index::{PdscRef, Pidx, Vidx};
use crate::pdsc::Package;
use crate::utils::parse::FromElem;

fn parse_vidx(body: Chunk) -> Result<Vidx, minidom::Error> {
    let string = String::from_utf8_lossy(body.as_ref());
    Vidx::from_string(string.borrow())
}

fn into_uri(Pidx { url, vendor, .. }: Pidx) -> String {
    format!("{}{}.pidx", url, vendor)
}

pub trait DownloadConfig {
    fn pack_store(&self) -> PathBuf;
}

pub trait IntoDownload {
    fn into_uri(&self) -> Result<Url, UrlError>;
    fn into_fd<D: DownloadConfig>(&self, _: &D) -> PathBuf;
}

impl IntoDownload for PdscRef {
    fn into_uri(&self) -> Result<Url, UrlError> {
        let &PdscRef {
            ref url,
            ref vendor,
            ref name,
            ..
        } = self;
        let uri = if url.ends_with('/') {
            format!("{}{}.{}.pdsc", url, vendor, name)
        } else {
            format!("{}/{}.{}.pdsc", url, vendor, name)
        }
        .parse()?;
        Ok(uri)
    }

    fn into_fd<D: DownloadConfig>(&self, config: &D) -> PathBuf {
        let &PdscRef {
            ref vendor,
            ref name,
            ref version,
            ..
        } = self;
        let mut filename = config.pack_store();
        let pdscname = format!("{}.{}.{}.pdsc", vendor, name, version);
        filename.push(pdscname);
        filename
    }
}

impl<'a> IntoDownload for &'a Package {
    fn into_uri(&self) -> Result<Url, UrlError> {
        let &Package {
            ref name,
            ref vendor,
            ref url,
            ref releases,
            ..
        } = *self;
        let version: &str = releases.latest_release().version.as_ref();
        let uri = if url.ends_with('/') {
            format!("{}{}.{}.{}.pack", url, vendor, name, version)
        } else {
            format!("{}/{}.{}.{}.pack", url, vendor, name, version)
        }
        .parse()?;
        Ok(uri)
    }

    fn into_fd<D: DownloadConfig>(&self, config: &D) -> PathBuf {
        let &Package {
            ref name,
            ref vendor,
            ref releases,
            ..
        } = *self;
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
    fn for_file(&self, _: &str) -> Self {}
}

pub struct DownloadContext<'a, Conf, Prog>
where
    Conf: DownloadConfig,
    Prog: DownloadProgress + 'a,
{
    config: &'a Conf,
    prog: Prog,
    client: Client,
}

impl<'a, Conf, Prog> DownloadContext<'a, Conf, Prog>
where
    Conf: DownloadConfig,
    Prog: DownloadProgress + 'a,
{
    pub fn new(config: &'a Conf, prog: Prog) -> Result<Self, Error> {
        let client = ClientBuilder::new()
            .use_rustls_tls()
            .use_sys_proxy()
            .redirect(RedirectPolicy::limited(5))
            .build()?;
        Ok(DownloadContext {
            config,
            prog,
            client,
        })
    }

    fn download_file(
        &'a self,
        source: Url,
        dest: PathBuf,
    ) -> Box<dyn Future<Item = (), Error = Error> + 'a> {
        if !dest.exists() {
            dest.parent().map(create_dir_all);
            Box::new(
                self.client
                    .get(source)
                    .send()
                    .from_err()
                    .and_then(move |res| {
                        let temp = dest.with_extension("part");
                        let fdf = result(OpenOptions::new().write(true).create(true).open(&temp))
                            .from_err();
                        fdf.and_then(move |mut fd| {
                            res.into_body()
                                .from_err::<Error>()
                                .for_each(move |bytes| {
                                    self.prog.progress(bytes.len());
                                    fd.write_all(bytes.as_ref())?;
                                    Ok(())
                                })
                                .then(move |_| {
                                    rename(&temp, &dest)?;
                                    Ok(())
                                })
                        })
                    }),
            )
        } else {
            Box::new(ok(()))
        }
    }

    pub fn download_stream<F, DL>(
        &'a self,
        stream: F,
    ) -> Box<dyn Stream<Item = PathBuf, Error = Error> + 'a>
    where
        F: Stream<Item = DL, Error = Error> + 'a,
        DL: IntoDownload + 'a,
    {
        let streaming_pathbuffs = stream
            .collect()
            .map(move |to_dl| {
                let len = to_dl.len();
                self.prog.size(len);
                iter_ok(to_dl).map(move |from| {
                    let dest = from.into_fd(self.config);
                    let source = from.into_uri();
                    result(source)
                        .from_err()
                        .and_then(move |source| {
                            self.download_file(source.clone(), dest.clone())
                                .then(move |res| {
                                    self.prog.complete();
                                    match res {
                                        Ok(_) => Ok(Some(dest)),
                                        Err(e) => {
                                            log::error!(
                                                "download of {:?} failed: {}",
                                                source,
                                                e
                                            );
                                            Ok(None)
                                        }
                                    }
                                })
                        })
                })
            })
            .flatten_stream();
        Box::new(streaming_pathbuffs.buffer_unordered(32).filter_map(|x| x))
    }

    fn download_vidx<I: Into<String>>(
        &'a self,
        vidx_ref: I,
    ) -> impl Future<Item = Result<Vidx, minidom::Error>, Error = Error> + 'a {
        let vidx = vidx_ref.into();
        result(vidx.parse())
            .from_err()
            .and_then(move |uri: Url| {
                self.client
                    .get(uri)
                    .send()
                    .map(Response::into_body)
                    .flatten_stream()
                    .concat2()
                    .from_err()
            })
            .map(parse_vidx)
    }

    pub(crate) fn download_vidx_list<I>(
        &'a self,
        list: I,
    ) -> impl Stream<Item = Option<Vidx>, Error = reqwest::Error> + 'a
    where
        I: IntoIterator + 'a,
        <I as IntoIterator>::Item: Into<String>,
    {
        futures_unordered(list.into_iter().map(|vidx_ref| {
            let vidx = vidx_ref.into();
            self.download_vidx(vidx.clone()).then(move |r| {
                match r {
                    Ok(Ok(r)) => Ok(Some(r)),
                    Ok(Err(e)) => {
                        log::error!("{}", format!("{}", e).replace("uri", &vidx));
                        Ok(None)
                    }
                    Err(e) => {
                        log::error!("{}", format!("{}", e).replace("uri", &vidx));
                        Ok(None)
                    }
                }
            })
        }))
    }

    pub(crate) fn flatmap_pdscs(
        &'a self,
        Vidx {
            vendor_index,
            pdsc_index,
            ..
        }: Vidx,
    ) -> impl Stream<Item = PdscRef, Error = Error> + 'a {
        let pidx_urls = vendor_index.into_iter().map(into_uri);
        let job = self
            .download_vidx_list(pidx_urls)
            .filter_map(|vidx| vidx.map(|v| iter_ok(v.pdsc_index.into_iter())))
            .flatten();
        iter_ok(pdsc_index.into_iter()).chain(job)
    }
}
