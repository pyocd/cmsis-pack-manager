use futures::prelude::*;
use futures::{Stream, Poll, Async};
use futures::stream::{iter_ok, iter_result, futures_unordered, FuturesUnordered};
use hyper::{self, Client, Response, Body, Chunk, Uri, StatusCode};
use hyper::client::{FutureResponse, Connect};
use hyper::header::Location;
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Core;
use std::borrow::Borrow;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::iter::Iterator;
use std::path::PathBuf;
use clap::{App, ArgMatches, SubCommand};
use slog::Logger;

use minidom;

use super::{PdscRef, Vidx, Pidx};
use parse::FromElem;
use config::{self, Config};


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

trait ClientRedirExt<C> where C: Connect {
    fn redirectable<'a>(
        &'a self, uri: Uri, logger: &'a Logger
    ) -> Box<Future<Item=Response, Error=hyper::Error> + 'a>;
}

impl<C: Connect> ClientRedirExt<C> for Client<C, Body> {
    fn redirectable<'a>(
        &'a self, mut uri: Uri, logger: &'a Logger
    ) -> Box<Future<Item=Response, Error=hyper::Error> + 'a> {
        Box::new(
            async_block!{
                let mut urls = Vec::new();
                loop {
                    urls.push(uri.clone());
                    let res = await!(self.get(uri))?;
                    match res.status() {
                        StatusCode::MovedPermanently |
                        StatusCode::Found |
                        StatusCode::SeeOther |
                        StatusCode::TemporaryRedirect |
                        StatusCode::PermanentRedirect => {
                            let mut new_uri: Uri = res.headers()
                                .get::<Location>()
                                .unwrap_or(&Location::new(""))
                                .parse()?;
                            if let Some(ref old_uri) = urls.last() {
                                if new_uri.authority().is_none() {
                                    if let Some(authority) = old_uri.authority() {
                                        new_uri = format!("{}{}", authority, old_uri).parse()?
                                    }
                                }
                                debug!(logger, "Redirecting from {} to {}", old_uri, new_uri);
                            }
                            uri = new_uri;
                        }
                        _ => {
                            return Ok(res);
                        }
                    }
                }
            })
    }
}

fn download_vidx<'a, C: Connect, I: Into<String>>(
    client: &'a Client<C, Body>, vidx_ref: I, logger:&'a Logger,
) -> impl Future<Item=Vidx, Error=Error> + 'a {
    let vidx = vidx_ref.into();
    async_block!{
        let uri = vidx.parse()?;
        let body = await!(
            client.redirectable(uri, logger)
                .map(Response::body)
                .flatten_stream()
                .concat2())?;
        parse_vidx(body, logger)
    }
}

fn download_vidx_list<'a, C, I>(
    list: I,
    client: &'a Client<C, Body>,
    logger: &'a Logger,
) -> impl Stream<Item = Vidx, Error = Error> + 'a
where
    C: Connect,
    I: IntoIterator + 'a,
    <I as IntoIterator>::Item: Into<String>,
{
    futures_unordered(list.into_iter().map(
        |vidx_ref| download_vidx(client, vidx_ref, logger)
    ))
}

fn parse_vidx(body: Chunk, logger: &Logger) -> Result<Vidx> {
    let string = String::from_utf8_lossy(body.as_ref());
    Vidx::from_string(string.borrow(), logger).map_err(Error::from)
}

fn into_uri(Pidx {url, vendor, ..}: Pidx) -> String {
    format!("{}{}.pidx", url, vendor)
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
    let pidx_urls = vendor_index.into_iter().map(into_uri);
    let job = download_vidx_list(pidx_urls, client, logger)
        .map(|vidx| iter_ok(vidx.pdsc_index.into_iter()))
        .flatten();
    iter_ok(pdsc_index.into_iter()).chain(job)
}

fn make_uri(
    &PdscRef {
        ref url,
        ref vendor,
        ref name,
        ..
    }: &PdscRef,
) -> Result<Uri> {
    let uri = if url.ends_with('/') {
        format!("{}{}.{}.pdsc", url, vendor, name)
    } else {
        format!("{}/{}.{}.pdsc", url, vendor, name)
    }.parse()?;
    Ok(uri)
}

fn make_fd(
    config: &Config,
    &PdscRef {
        ref vendor,
        ref name,
        ref version,
        ..
    }: &PdscRef,
) -> Result<PathBuf> {
    let pdscname = format!("{}.{}.{}.pdsc", vendor, name, version);
    let filename = config.pack_store.place_data_file(&pdscname)?;
    Ok(filename)
}

fn download_pdsc<'a, C: Connect>(
    config: &'a Config,
    pdsc_ref: PdscRef,
    client: &'a Client<C, Body>,
    logger: &'a Logger,
) -> impl Future<Item = Option<PathBuf>, Error = Error> + 'a {
    async_block!{
        let filename = make_fd(config, &pdsc_ref)?;
        if filename.exists() {
            return Ok(None);
        }
        let uri = make_uri(&pdsc_ref)?;
        let PdscRef{vendor, name, version, ..} = pdsc_ref;
        info!(logger, "Updating package {}::{} to version {}", vendor, name, version);
        let mut fd = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&filename)?;
        let response = await!(client.redirectable(uri, logger))?;
        #[async]
        for bytes in response.body() {
            fd.write_all(bytes.as_ref())?;
        }
        Ok(Some(filename))
    }
}

fn download_pdsc_stream<'a, F, C>(
    config: &'a Config,
    stream: F,
    client: &'a Client<C, Body>,
    logger: &'a Logger,
) -> impl Stream<Item = Option<PathBuf>, Error = Error> + 'a
where
    F: Stream<Item = PdscRef, Error = Error> + 'a,
    C: Connect,
{
    stream
        .map(move |pdsc_ref| download_pdsc(config, pdsc_ref, client, logger))
        .buffer_unordered(32)
}

fn id<T>(slf: T) -> T {
    slf
}

// This will "trick" the borrow checker into thinking that the lifetimes for
// client and core are at least as big as the lifetime for pdscs, which they actually are
fn update_inner<C, I>(
    config: &Config,
    vidx_list: I,
    core: &mut Core,
    client: &Client<C, Body>,
    logger: &Logger,
) -> Result<Vec<PathBuf>>
where
    C: Connect,
    I: IntoIterator<Item = String>,
{
    let parsed_vidx = download_vidx_list(vidx_list, client, logger);
    let pdsc_list = parsed_vidx
        .map(|vidx| flatmap_pdscs(vidx, client, logger))
        .flatten();
    let pdscs = download_pdsc_stream(config, pdsc_list, client, logger);
    core.run(pdscs.filter_map(id).collect())
}

/// Flatten a list of Vidx Urls into a list of updated CMSIS packs
pub fn update<I>(config: &Config, vidx_list: I, logger: &Logger) -> Result<Vec<PathBuf>>
where
    I: IntoIterator<Item = String>,
{
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

#[no_mangle]
pub extern fn update_pdsc_index()  {
    extern crate slog_term;
    extern crate slog_async;
    use slog::Drain;
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let log = Logger::root(drain, o!());
    let conf = Config::new().unwrap();
    let vidx_list = conf.read_vidx_list(&log);
    let updated = update(&conf, vidx_list, &log).unwrap();
}

pub fn update_command<'a>(conf: &Config, _: &ArgMatches<'a>, logger: &Logger) -> Result<()> {
    let vidx_list = conf.read_vidx_list(&logger);
    for url in vidx_list.iter() {
        info!(logger, "Updating registry from `{}`", url);
    }
    let updated = update(conf, vidx_list, logger)?;
    let num_updated = updated.iter().map(|_| 1).sum::<u32>();
    match num_updated {
        0 => {
            info!(logger, "Already up to date");
        }
        1 => {
            info!(logger, "Updated 1 package");
        }
        _ => {
            info!(logger, "Updated {} package", num_updated);
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
}
