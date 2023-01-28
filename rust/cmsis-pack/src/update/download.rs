use std::fs::{create_dir_all, rename, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Error};
use futures::prelude::*;
use futures::stream::futures_unordered::FuturesUnordered;
use reqwest::{redirect, Url};
use reqwest::{Client, ClientBuilder, Response};
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

use crate::pack_index::{PdscRef, Vidx};
use crate::pdsc::Package;
use crate::utils::parse::FromElem;
use futures::StreamExt;
use std::collections::HashMap;

const CONCURRENCY : usize = 32;
const HOST_LIMIT : usize = 6;
const MAX_RETRIES : usize = 3;

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


async fn save_response(response: Response, dest: PathBuf) -> Result<(usize, PathBuf), Error> {
    let temp = dest.with_extension("part");
    let file = OpenOptions::new().write(true).create(true).open(&temp);

    let mut file = match file {
        Err(err) => return Err(anyhow!(err.to_string())),
        Ok(f) => f,
    };

    let mut fsize: usize = 0;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bytes) => {
                fsize += bytes.len();

                if let Err(err) = file.write_all(bytes.as_ref()) {
                    let _ = std::fs::remove_file(temp);
                    return Err(anyhow!(err.to_string()));
                }
            }
            Err(err) => {
                let _ = std::fs::remove_file(temp);
                return Err(anyhow!(err.to_string()));
            }
        }
    }
    if let Err(err) = rename(&temp, &dest) {
        let _ = std::fs::remove_file(temp);
        return Err(anyhow!(err.to_string()));
    }
    Ok((fsize, dest))
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

    pub async fn download_iterator<I>(&'a self, iter: I) -> Vec<PathBuf>
    where
        I: IntoIterator + 'a,
        <I as IntoIterator>::Item: IntoDownload,
    {
        let mut to_dl: Vec<(Url, String, PathBuf)> = iter
            .into_iter()
            .filter_map(|i| {
                if let Ok(uri) = i.into_uri() {
                    let c = uri.clone();
                    if let Some(host) = c.host_str() {
                        Some((uri, host.to_string(), i.into_fd(self.config)))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        self.prog.size(to_dl.len());

        let mut hosts: HashMap<String, usize> = HashMap::new();
        let mut results : Vec<PathBuf> = vec![];
        let mut started : usize = 0;
        let mut handles: Vec<JoinHandle<(String, usize, Option<PathBuf>)>> = vec![];

        while !to_dl.is_empty() || !handles.is_empty() {
            let mut wait_list: Vec<(Url, String, PathBuf)> = vec![];
            let mut next: Vec<JoinHandle<(String, usize, Option<PathBuf>)>> = vec![];

            while let Some(handle) = handles.pop() {
                if handle.is_finished() {
                    let r = handle.await.unwrap();
                    *hosts.entry(r.0).or_insert(1) -= 1;
                    started -= 1;
                    self.prog.progress(r.1);
                    self.prog.complete();
                    if let Some(path) = r.2 {
                        results.push(path);
                    }
                } else {
                    next.push(handle);
                }
            }

            while ! to_dl.is_empty() && started < CONCURRENCY {
                let from = to_dl.pop().unwrap();
                let host = from.1.clone();
                let entry = hosts.entry(host).or_insert(0);
                if *entry >= HOST_LIMIT  {
                    wait_list.push(from);
                } else {
                    let source = from.0.clone();
                    let host = from.1.clone();
                    let dest = from.2.clone();
                    if dest.exists() {
                        results.push(dest);
                    } else {
                        let client = self.client.clone();
                        let handle: JoinHandle<(String, usize, Option<PathBuf>)> = tokio::spawn(async move {
                            dest.parent().map(create_dir_all);
                            let res = client.get(source.clone()).send().await;
                            let res: Result<(usize, PathBuf), Error> = match res {
                                Ok(r) => {
                                    let rc = r.status().as_u16();
                                    if rc >= 400 {
                                        Err(anyhow!(format!("Response code in invalid range: {}", rc).to_string()))
                                    } else {
                                        save_response(r, dest).await
                                    }
                                },
                                Err(err) => {
                                    Err(anyhow!(err.to_string()))
                                },
                            };
                            match res {
                                Ok(r) => {
                                    (host, r.0, Some(r.1))
                                },
                                Err(err) => {
                                    log::error!("download of {:?} failed: {}", source, err);
                                    (host, 0, None)
                                }
                            }
                        });
                        handles.push(handle);
                        started += 1;
                        *entry += 1;
                    }
                }
            }

            for w in wait_list {
                to_dl.push(w);
            }

            for w in next {
                handles.push(w);
            }
            sleep(Duration::from_millis(100)).await;
        }

        results
    }

    pub(crate) async fn update_vidx<I>(&'a self, list: I) -> Result<Vec<PathBuf>, Error>
    where
        I: IntoIterator + 'a,
        <I as IntoIterator>::Item: Into<String>,
    {
        let mut downloaded: HashMap<String, bool> = HashMap::new();
        let mut failures: HashMap<String, usize> = HashMap::new();
        let mut urls: Vec<String> = list.into_iter().map(|x| x.into()).collect();
        let mut vidxs: Vec<Vidx> = Vec::new();
        loop {
            // Remove from list all duplicate URLs and those already downloaded
            urls.dedup();
            urls = urls
                .into_iter()
                .filter(|u| !*downloaded.get(u).unwrap_or(&false))
                .collect();

            // TODO: Make this section asynchronous
            let mut next: Vec<String> = Vec::new();
            for url in urls {
                match self.download_vidx(url.clone()).await {
                    Ok(t) => {
                        log::info!("Downloaded {}", url);
                        downloaded.insert(url, true);
                        for v in &t.vendor_index {
                            let u = format!("{}{}.pidx", v.url, v.vendor);
                            if !downloaded.contains_key(&u) {
                                downloaded.insert(u.clone(), false);
                                next.push(u);
                            }
                        }
                        vidxs.push(t);
                    }
                    Err(_err) => {
                        let tries = failures.entry(url.clone()).or_insert(0);
                        *tries += 1;
                        if *tries < MAX_RETRIES {
                            next.push(url);
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

    #[allow(dead_code)]
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
