use futures::{Stream, Poll, Async};
use futures::stream::{iter, FuturesUnordered};
use hyper::{self, Client, Response, Body, Chunk, Uri, StatusCode};
use hyper::client::{FutureResponse, Connect};
use hyper::header::Location;
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Core;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;
use clap::{App, ArgMatches, SubCommand};
use slog::Logger;

use minidom;

use super::{PdscRef, Vidx, Pidx};
use parse::FromElem;
use config::{self, Config};

static PIDX_SUFFIX: &'static str = ".pidx";

error_chain!{
    links{
        MinidomErr(minidom::Error, minidom::ErrorKind);
        ConfigErr(config::Error, config::ErrorKind);
    }
    foreign_links{

        HttpErr(hyper::Error);
        UriErr(hyper::error::UriError);
        IOErr(io::Error);
    }
}

future_chain!{}

struct Redirect<'a, C>
where
    C: Connect,
{
    urls: Vec<Uri>,
    current: FutureResponse,
    client: &'a Client<C, Body>,
    logger: Logger,
}

impl<'a, C> Redirect<'a, C>
where
    C: Connect,
{
    fn new(client: &'a Client<C, Body>, uri: Uri, logger: Logger) -> Self {
        let current = client.get(uri.clone());
        Self {
            urls: vec![uri],
            current,
            client,
            logger,
        }
    }
}

impl<'a, C> Future for Redirect<'a, C>
where
    C: Connect,
{
    type Item = Response;
    type Error = hyper::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match self.current.poll()? {
                Async::NotReady => {
                    return Ok(Async::NotReady);
                }
                Async::Ready(res) => {
                    match res.status() {
                        StatusCode::MovedPermanently |
                        StatusCode::Found |
                        StatusCode::SeeOther |
                        StatusCode::TemporaryRedirect |
                        StatusCode::PermanentRedirect => {
                            let mut uri: Uri = res.headers()
                                .get::<Location>()
                                .unwrap_or(&Location::new(""))
                                .parse()?;
                            if let Some(old_uri) = self.urls.last() {
                                if uri.authority().is_none() {
                                    if let Some(authority) = old_uri.authority() {
                                        uri = format!("{}{}", authority, uri).parse()?
                                    }
                                }
                                debug!(self.logger, "Redirecting from {} to {}", old_uri, uri);
                            }
                            self.urls.push(uri.clone());
                            self.current = self.client.get(uri);
                        }
                        _ => {
                            return Ok(Async::Ready(res));
                        }
                    }
                }
            }
        }
    }
}

fn download_vidx_list<'a, C>(
    list: Vec<String>,
    client: &'a Client<C, Body>,
    logger: &'a Logger
) -> impl Stream<Item = Vidx, Error = Error> + 'a
where
    C: Connect,
{
    let mut job = FuturesUnordered::new();
    for vidx_ref in list {
        match vidx_ref.parse() {
            Ok(uri) => {
                let child_log = logger.new(o!());
                job.push(
                    Redirect::new(client, uri, logger.new(o!()))
                        .map(Response::body)
                        .flatten_stream()
                        .concat2()
                        .map_err(Error::from)
                        .and_then(move |body| {
                            parse_vidx(body, &child_log)
                        }),
                );
            }
            Err(e) => error!(logger, "Url {} did not parse {}", vidx_ref, e),
        }
    }
    Box::new(job) as Box<Stream<Item = _, Error = _>>
}

fn parse_vidx(body: Chunk, logger: &Logger) -> Result<Vidx> {
    let string = String::from_utf8_lossy(body.as_ref()).into_owned();
    Vidx::from_string(string.as_str(), logger).map_err(Error::from)
}

fn stream_pdscs(body: Chunk, logger: &Logger) -> impl Iterator<Item = Result<PdscRef>> {
    parse_vidx(body, logger)
        .into_iter()
        .flat_map(|vidx| vidx.pdsc_index.into_iter())
        .map(Ok::<_, Error>)
}

fn flatmap_pdscs<'a, C>(
    Vidx {
        vendor_index,
        pdsc_index,
        ..
    }: Vidx,
    client: &'a Client<C, Body>,
    logger: &'a Logger,
) -> impl Stream<Item = PdscRef, Error = Error> + 'a
where
    C: Connect,
{
    let mut job = FuturesUnordered::new();
    for Pidx { url, vendor, .. } in vendor_index {
        let urlname = format!("{}{}{}", url, vendor, PIDX_SUFFIX);
        match urlname.parse() {
            Ok(uri) => {
                let l = logger.new(o!());
                let work = Redirect::new(client, uri, logger.new(o!()))
                    .map(Response::body)
                    .flatten_stream()
                    .concat2()
                    .map(move |body| {
                        stream_pdscs(body, &l)
                    })
                    .from_err::<Error>();
                job.push(work)
            }
            Err(e) => error!(logger, "Url {} did not parse {}", urlname, e),
        }
    }
    Box::new(iter(pdsc_index.into_iter().map(Ok::<_, Error>)).chain(
        job.map(iter).flatten(),
    )) as Box<Stream<Item = _, Error = _>>
}

fn make_uri_fd_pair(
    config: &Config,
    PdscRef {url, vendor, name, version, ..}: PdscRef,
    logger: &Logger,
) -> Result<Option<(Uri, String, PathBuf)>> {

    let uri = if url.ends_with('/') {
        format!("{}{}.{}.pdsc", url, vendor, name)
    } else {
        format!("{}/{}.{}.pdsc", url, vendor, name)
    }.parse()?;
    let pdscname = format!("{}.{}.{}.pdsc",
                           vendor,
                           name,
                           version);
    let filename = config.pack_store.place_data_file(&pdscname)?;
    if filename.exists() {
        Ok(None)
    } else {
        info!(logger, "Updating pdsc `{}`", pdscname);
        Ok(Some((uri, url, filename)))
    }
}

fn id<T>(slf: T) -> T {
    slf
}

fn download_pdscs<'a, F, C>(
    config: &'a Config,
    stream: F,
    client: &'a Client<C, Body>,
    logger: &'a Logger,
) -> impl Stream<Item = Option<PathBuf>, Error = Error> + 'a
where
    F: Stream<Item = PdscRef, Error = Error> + 'a,
    C: Connect,
{
    Box::new(
        stream
            .and_then(move |pdscref| make_uri_fd_pair(config, pdscref, logger))
            .filter_map(id)
            .map(move |(uri, url, filename)| {
                Redirect::new(client, uri, logger.new(o!()))
                    .map(Response::body)
                    .flatten_stream()
                    .concat2()
                    .map_err(Error::from)
                    .and_then(move |bytes| {
                        let mut fd = OpenOptions::new()
                            .write(true)
                            .create(true)
                            .open(&filename)
                            .map_err(Error::from)?;
                        fd.write_all(bytes.as_ref()).map_err(Error::from).map(|_| {
                            Some(filename)
                        })
                    })
                    .or_else(move |e| {
                        error!(logger, "HTTP request for {} failed with {}", url, e);
                        Ok::<_, Error>(None)
                    })
            })
            .buffer_unordered(32),
    ) as Box<Stream<Item = _, Error = _>>
}

// This will "trick" the borrow checker into thinking that the lifetimes for
// client and core are at least as big as the lifetime for pdscs, which they actually are
fn update_inner<C>(
    config: &Config,
    vidx_list: Vec<String>,
    core: &mut Core,
    client: &Client<C, Body>,
    logger: &Logger
) -> Result<Vec<PathBuf>>
where
    C: Connect,
{
    let parsed_vidx = download_vidx_list(vidx_list, client, logger);
    let pdsc_list = parsed_vidx
        .map(|vidx| flatmap_pdscs(vidx, client, logger))
        .flatten();
    let pdscs = download_pdscs(config, pdsc_list, client, logger);
    core.run(pdscs.filter_map(id).collect())
}

/// Flatten a list of Vidx Urls into a list of updated CMSIS packs
pub fn update(config: &Config,
              vidx_list: Vec<String>,
              logger: &Logger) -> Result<Vec<PathBuf>> {
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let client = Client::configure()
        .keep_alive(true)
        .connector(HttpsConnector::new(4, &handle).unwrap())
        .build(&handle);
    update_inner(config, vidx_list, &mut core, &client, logger)
}

pub fn update_args<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("update")
        .about("Update CMSIS PDSC files for indexing")
        .version("0.1.0")
}

pub fn update_command<'a>(conf: &Config,
                          _: &ArgMatches<'a>,
                          logger: &Logger) -> Result<()> {
    let vidx_list = conf.read_vidx_list(logger.clone());
    for url in vidx_list.iter() {
        info!(logger, "Updating registry from `{}`", url);
    }
    let updated = update(conf, vidx_list, logger)?;
    if !updated.is_empty() {
        for pdsc_name in updated.iter().filter_map(|pb| {
            pb.file_name().and_then(|osstr| osstr.to_str())
        })
        {
            info!(logger, "Updated {}", pdsc_name);
        }
    }
    Ok(())
}
