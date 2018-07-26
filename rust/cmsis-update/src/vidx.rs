use std::borrow::Borrow;

use failure::Error;
use futures::prelude::{await, async_block, Future};
use futures::Stream;
use futures::stream::{futures_unordered, iter_ok};
use hyper::{self, Body, Chunk, Client, Response};
use hyper::client::Connect;
use minidom;
use slog::Logger;

use pack_index::{PdscRef, Pidx, Vidx};
use utils::parse::FromElem;

use redirect::ClientRedirExt;

fn download_vidx<'a, C: Connect, I: Into<String>>(
    client: &'a Client<C, Body>,
    vidx_ref: I,
    logger: &'a Logger,
) -> impl Future<Item = Result<Vidx, minidom::Error>, Error = hyper::Error> + 'a {
    let vidx = vidx_ref.into();
    async_block!{
        let uri = vidx.parse()?;
        let body = await!(
            client.redirectable(uri, logger)
                .map(Response::body)
                .flatten_stream()
                .concat2())?;
        Ok(parse_vidx(body, logger))
    }
}

pub(crate) fn download_vidx_list<'a, C, I>(
    list: I,
    client: &'a Client<C, Body>,
    logger: &'a Logger,
) -> impl Stream<Item = Result<Vidx, minidom::Error>, Error = hyper::Error> + 'a
where
    C: Connect,
    I: IntoIterator + 'a,
    <I as IntoIterator>::Item: Into<String>,
{
    futures_unordered(
        list.into_iter()
            .map(|vidx_ref| download_vidx(client, vidx_ref, logger)),
    )
}

fn parse_vidx(body: Chunk, logger: &Logger) -> Result<Vidx, minidom::Error> {
    let string = String::from_utf8_lossy(body.as_ref());
    Vidx::from_string(string.borrow(), logger)
}

fn into_uri(Pidx { url, vendor, .. }: Pidx) -> String {
    format!("{}{}.pidx", url, vendor)
}

pub(crate) fn flatmap_pdscs<'a, C>(
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
        .filter_map(|vidx| match vidx {
            Ok(v) => Some(iter_ok(v.pdsc_index.into_iter())),
            Err(_) => None,
        })
        .flatten();
    iter_ok(pdsc_index.into_iter()).chain(job)
}
