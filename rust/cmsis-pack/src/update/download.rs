use std::borrow::Borrow;
use std::fs::{create_dir_all, rename, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Error};
use futures::prelude::*;
use futures::stream::futures_unordered::FuturesUnordered;
use reqwest::{redirect, Url};
use reqwest::{Client, ClientBuilder, Response};

use crate::pack_index::{PdscRef, Pidx, Vidx};
use crate::pdsc::Package;
use crate::utils::parse::FromElem;
use bytes::Bytes;
use futures::StreamExt;
use std::collections::HashMap;

fn parse_vidx(body: Bytes) -> Result<Vidx, Error> {
    let string = String::from_utf8_lossy(body.as_ref());
    Vidx::from_string(string.borrow())
}

fn pdsc_url(pdsc: &mut PdscRef) -> String {
    if pdsc.url.ends_with('/') {
        format!("{}{}.{}.pdsc", pdsc.url, pdsc.vendor, pdsc.name)
    } else {
        format!("{}/{}.{}.pdsc", pdsc.url, pdsc.vendor, pdsc.name)
    }
}

pub trait DownloadConfig {
    fn pack_store(&self) -> PathBuf;
}

pub trait IntoDownload {
    fn into_uri(&self) -> Result<Url, Error>;
    fn into_fd<D: DownloadConfig>(&self, _: &D) -> PathBuf;
}

impl IntoDownload for PdscRef {
    fn into_uri(&self) -> Result<Url, Error> {
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
    fn into_uri(&self) -> Result<Url, Error> {
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
            .redirect(redirect::Policy::limited(5))
            .build()?;

        Ok(DownloadContext {
            config,
            prog,
            client,
        })
    }

    async fn save_response(&'a self, response: Response, dest: PathBuf) -> Result<PathBuf, Error> {
        let temp = dest.with_extension("part");
        let file = OpenOptions::new().write(true).create(true).open(&temp);

        let mut file = match file {
            Err(err) => return Err(anyhow!(err.to_string())),
            Ok(f) => f,
        };

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    self.prog.progress(bytes.len());

                    if let Err(err) = file.write_all(bytes.as_ref()) {
                        std::fs::remove_file(temp);
                        return Err(anyhow!(err.to_string()));
                    }
                }
                Err(err) => {
                    std::fs::remove_file(temp);
                    return Err(anyhow!(err.to_string()));
                }
            }
        }
        if let Err(err) = rename(&temp, &dest) {
            std::fs::remove_file(temp);
            return Err(anyhow!(err.to_string()));
        }
        Ok(dest)
    }

    async fn download_file(&'a self, source: Url, dest: PathBuf) -> Result<PathBuf, Error> {
        if dest.exists() {
            return Ok(dest);
        }
        dest.parent().map(create_dir_all);
        let res = self.client.get(source).send().await;

        match res {
            Ok(r) => self.save_response(r, dest).await,
            Err(err) => Err(anyhow!(err.to_string())),
        }
    }

    pub async fn download_iterator<I>(&'a self, iter: I) -> Vec<PathBuf>
    where
        I: IntoIterator + 'a,
        <I as IntoIterator>::Item: IntoDownload,
    {
        let to_dl: Vec<(Url, PathBuf)> = iter
            .into_iter()
            .filter_map(|i| {
                if let Ok(uri) = i.into_uri() {
                    Some((uri, i.into_fd(self.config)))
                } else {
                    None
                }
            })
            .collect();
        self.prog.size(to_dl.len());

        let v = futures::stream::iter(to_dl.into_iter().map(|from| async move {
            let r = self.download_file(from.0.clone(), from.1.clone()).await;
            self.prog.complete();
            match r {
                Ok(p) => Some(p),
                Err(e) => {
                    log::error!("download of {:?} failed: {}", from.0.clone(), e);
                    None
                }
            }
        }))
        .buffer_unordered(32)
        .collect::<Vec<Option<PathBuf>>>()
        .await;
        v.into_iter().filter_map(|x| x).collect::<Vec<PathBuf>>()
    }

    pub(crate) async fn update_vidx<I>(&'a self, list: I) -> Result<Vec<PathBuf>, Error>
    where
        I: IntoIterator + 'a,
        <I as IntoIterator>::Item: Into<String>,
    {
        let mut downloaded: HashMap<String, i8> = HashMap::new();
        let mut urls: Vec<String> = list.into_iter().map(|x| x.into()).collect();
        let mut vidxs: Vec<Vidx> = Vec::new();
        loop {
            // Remove from list all duplicate URLs and those already downloaded
            urls.dedup();
            urls = urls
                .into_iter()
                .filter(|u| downloaded.get(u).unwrap_or(&0) < &1)
                .collect();

            // TODO: Make this section asynchronous
            let mut next: Vec<String> = Vec::new();
            for url in &urls {
                match self.download_vidx(url.clone()).await {
                    Ok(t) => {
                        log::info!("Downloaded {}", url);
                        downloaded.insert(url.to_string(), 1);
                        for v in &t.vendor_index {
                            let u = format!("{}{}.pidx", v.url, v.vendor);
                            if !downloaded.contains_key(&u) {
                                downloaded.insert(u.to_string(), 0);
                                next.push(u);
                            }
                        }
                        vidxs.push(t);
                    }
                    Err(_err) => {
                        let r = downloaded.get(&url.to_string()).unwrap_or(&0);
                        if r > &-3 {
                            next.push(url.clone());
                            downloaded.insert(url.to_string(), r - &1);
                        }
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            urls = next;
        }

        let mut pdscs: Vec<PdscRef> = Vec::new();
        for mut v in vidxs {
            pdscs.append(&mut v.pdsc_index);
        }

        pdscs.dedup_by_key(|pdsc| pdsc_url(pdsc));
        log::info!("Found {} Pdsc entries", pdscs.len());

        Ok(self.download_iterator(pdscs.into_iter()).await)
    }

    pub(crate) async fn download_vidx<I: Into<String>>(
        &'a self,
        vidx_ref: I,
    ) -> Result<Vidx, Error> {
        let vidx = vidx_ref.into();
        let uri = vidx.parse::<Url>().unwrap();

        let req: reqwest::Response = self.client.get(uri).send().await?;
        Vidx::from_string(req.text().await?.as_str())
    }

    pub(crate) fn download_vidx_list<I>(&'a self, list: I) -> impl Stream<Item = Option<Vidx>> + 'a
    where
        I: IntoIterator + 'a,
        <I as IntoIterator>::Item: Into<String>,
    {
        list.into_iter()
            .map(|vidx_ref| {
                let vidx = vidx_ref.into();
                println!("{}", vidx);
                self.download_vidx(vidx.clone()).then(|r| async move {
                    match r {
                        Ok(v) => {
                            println!("{} success", vidx);
                            Some(v)
                        }
                        Err(e) => {
                            log::error!("{}", format!("{}", e).replace("uri", &vidx));
                            None
                        }
                    }
                })
            })
            .collect::<FuturesUnordered<_>>()
    }
}
