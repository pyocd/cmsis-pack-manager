use futures::{Stream, Poll, Async};
use futures::stream::{iter, FuturesUnordered};
use hyper::{self, Client, Response, Body, Chunk, Uri};
use hyper::client::{Connect};
use hyper_tls::HttpsConnector;
use tokio_core::reactor::{Core};
use std::fs::{OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

use minidom;

use super::{PdscRef, Vidx, Pidx};
use ::parse::FromElem;
use ::config::{self, Config};

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
    where C: Connect
{
    urls: Vec<Uri>,
    current: Box<Future<Item = Response, Error = hyper::Error>>,
    client: &'a Client<C, Body>
}

//impl<'a, C> Redirect<'a, C>
    //where C: Connect
//{
    //fn new<T>(current:  T, client: &'a Client<C, Body>) -> Self
        //where T: Future<Item = Response, Error = hyper::Error>
    //{
        //Self{
            //urls: Vec::new(),
            //current: Box::new(current),
            //client
        //}
    //}
//}

impl<'a, C> Future for Redirect<'a, C>
    where C: Connect
{
    type Item = Response;
    type Error = hyper::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.current.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(e),
            Ok(Async::Ready(res)) => Ok(Async::Ready(res))
        }
    }

}

pub fn download_vidx_list<C>
    (list: Vec<String>, client: &Client<C, Body>)
    -> impl Stream<Item = Vidx, Error = Error>
    where C: Connect
{
    let mut job = FuturesUnordered::new();
    for vidx_ref in list {
        match vidx_ref.parse() {
            Ok(uri) => {
                job.push(client.get(uri)
                         .map(Response::body)
                         .flatten_stream()
                         .concat2()
                         .map_err(Error::from)
                         .and_then(parse_vidx));
            }
            Err(e) => {
                error!("Url {} did not parse {}", vidx_ref, e)
            }
        }
    }
    job
}

fn parse_vidx(body: Chunk) -> Result<Vidx> {
    let string = String::from_utf8_lossy(body.as_ref()).into_owned();
    Vidx::from_string(string.as_str())
        .map_err(Error::from)
}

fn stream_pdscs(body: Chunk) -> impl Iterator<Item = Result<PdscRef>> {
    parse_vidx(body)
        .into_iter()
        .flat_map(|vidx| {vidx.pdsc_index.into_iter()})
        .map(Ok::<_, Error>)
}

pub fn flatten_to_pdsc_future<C>
    (Vidx{vendor_index, pdsc_index, ..}: Vidx, client: &Client<C, Body>)
     -> impl Stream<Item = PdscRef, Error = Error>
    where C: Connect
{
    let mut job = FuturesUnordered::new();
    for Pidx{url, vendor, ..} in vendor_index {
        let urlname = format!("{}{}{}", url, vendor, PIDX_SUFFIX);
        match urlname.parse() {
            Ok(uri) => {
                let work = client.get(uri)
                    .map(Response::body)
                    .flatten_stream()
                    .concat2()
                    .map(stream_pdscs)
                    .from_err::<Error>();
                job.push(work)
            }
            Err(e) => {
                error!("Url {} did not parse {}", urlname, e)
            }
        }
    }
    let stream = iter(pdsc_index.into_iter().map(Ok::<_, Error>))
        .chain(job.map(iter).flatten());
    stream
}

fn make_uri_fd_pair(config: &Config, PdscRef{url, vendor, name, version, ..}: PdscRef)
                    -> Result<Option<(Uri, String, PathBuf)>> {
    let uri = format!("{}{}.{}.pdsc", url, vendor, name)
        .parse()?;
    let filename =
        config.pack_store.place_data_file(
            format!("{}.{}.{}.pdsc",
                    vendor,
                    name,
                    version))?;
    if filename.exists() {
        info!("Skipping download of pdsc {} from vendor {} at version {}", name, vendor, version);
        Ok(None)
    } else {
        Ok(Some((uri, url, filename)))
    }
}

pub fn download_pdscs<'a, F, C>
    (config: &'a Config, stream: F, client: &'a Client<C, Body>)
     -> impl Stream<Item = Option<PathBuf>, Error = Error> + 'a
    where F: Stream<Item = PdscRef, Error = Error> + 'a,
          C: Connect
{
    stream
        .and_then( move |pdscref| make_uri_fd_pair(config, pdscref))
        .filter_map(|foo| foo)
        .map( move |(uri, url, filename)| {
            client.get(uri)
                .map(Response::body)
                .flatten_stream()
                .concat2()
                .map_err(Error::from)
                .and_then(move |bytes|{
                    let mut fd = OpenOptions::new()
                        .write(true)
                        .create(true)
                        .open(&filename)
                        .map_err(Error::from)?;
                    fd.write_all(bytes.as_ref())
                        .map_err(Error::from)
                        .map(|_| Some(filename))
                })
                .or_else(move |e|{
                    error!("HTTP request for {} failed with {}", url, e);
                    Ok::<_, Error>(None)
                })
        })
        .buffer_unordered(32)
}

// This will "trick" the borrow checker into thinking that the lifetimes for
// client and core are at least as big as the lifetime for pdscs, which they actually are
fn flatten_inner<C>
    (config: &Config, vidx_list: Vec<String>, core: &mut Core, client: Client<C, Body>)
     -> Result<Vec<PathBuf>>
    where C: Connect
{
    let parsed_vidx = download_vidx_list(vidx_list, &client);
    let pdsc_list = parsed_vidx.map(|vidx| {
        flatten_to_pdsc_future(vidx, &client)
    }).flatten();
    let pdscs = download_pdscs(config, pdsc_list, &client);
    core.run(pdscs.filter_map(|x| x).collect())
}

pub fn flatten(config: &Config, vidx_list: Vec<String>) -> Result<Vec<PathBuf>> {
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let client = Client::configure()
        .keep_alive(true)
        .connector(HttpsConnector::new(4, &handle).unwrap())
        .build(&handle);
    flatten_inner(config, vidx_list, &mut core, client)
}
