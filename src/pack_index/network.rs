use futures::{Stream};
use futures::future::{Executor};
use futures::stream::{iter, FuturesUnordered};
use futures::unsync::mpsc::{self, channel, unbounded, Receiver, UnboundedReceiver};
use hyper::{self, Client, Response, Body, Chunk, Uri};
use hyper::client::{Connect};
use hyper_tls::HttpsConnector;
use tokio_core::reactor::{Core, Handle};
use smallstring::SmallString;
use std::fs::{OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::iter::Map;
use std::vec::IntoIter;

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
        SinkErr(mpsc::SendError<Option<PathBuf>>);
        SinkErr2(mpsc::SendError<PdscRef>);
        HttpErr(hyper::Error);
        UriErr(hyper::error::UriError);
        IOErr(io::Error);
    }
}

future_chain!{}

fn void<T>(_: T) -> () { () }

pub fn download_vidx_list<C> (list: Vec<String>,
                              client: &Client<C, Body>,
                              core: Handle) -> Receiver<Vidx> {
    unimplemented!()
}

fn make_stream_pdscs(vendor: SmallString)
                     -> impl Fn(Chunk) -> impl Iterator<Item = Result<PdscRef>>
{
    move |body| {
        let string = String::from_utf8_lossy(body.as_ref()).into_owned();
        match Vidx::from_string(string.as_str()) {
            Ok(next_vidx) => {
                next_vidx.pdsc_index
            }
            Err(e) => {
                error!("failed to parse vendor index for {} because {}", vendor, e);
                Vec::new()
            }
        }.into_iter().map(Ok::<_, Error>)
    }
}

pub fn flatten_to_pdsc_future<C>
    (Vidx{vendor_index, pdsc_index, ..}: Vidx,
     client: &Client<C, Body>,
     core: Handle) -> Receiver<PdscRef>
    where C: Connect
{
    let mut job = FuturesUnordered::new();
    let (sender, reciever) = channel(vendor_index.len());
    for Pidx{url, vendor, ..} in vendor_index {
        let urlname = format!("{}{}{}", url, vendor, PIDX_SUFFIX);
        match urlname.parse() {
            Ok(uri) => {
                let stream_pdscs = make_stream_pdscs(vendor);
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
    core.execute(stream.forward(sender).map(void).map_err(void)).unwrap();
    reciever
}

fn make_uri_fd_pair(config: &Config, PdscRef{url, vendor, name, version, ..}: PdscRef)
                    -> Result<(Uri, String, PathBuf, SmallString)> {
    let uri = format!("{}{}.{}.pdsc", url, vendor, name)
        .parse()?;
    let filename =
        config.pack_store.place_data_file(
            format!("{}.{}.{}.pdsc",
                    vendor,
                    name,
                    version))?;
    Ok((uri, url, filename, name))
}

pub fn download_pdscs<F, C>
    (config: &Config, stream: F, client: &Client<C, Body>, core: &mut Core)
     -> impl Stream<Item = Option<PathBuf>, Error = ()>
    where F: Stream<Item = PdscRef, Error = ()>,
          C: Connect
{
    let (sender, reciever) = unbounded();
    let mut job = FuturesUnordered::new();
    core.run(stream
             .and_then( move |pdscref| make_uri_fd_pair(config, pdscref).map_err(void))
             .map(|(uri, url, filename, name)| {
                 job.push(client.get(uri)
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
                          }));
                 Ok::<_, ()>(())
             })
             .collect()
             .map(void)
    ).unwrap();
    core.execute(job.forward(sender).map(void).map_err(void)).unwrap();
    reciever
}

pub fn flatten_to_downloaded_pdscs(config: &Config, vidx: Vidx) -> Option<()> {
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let client = Client::configure()
        .keep_alive(true)
        .connector(HttpsConnector::new(4, &handle).unwrap())
        .build(&handle);
    let future = download_pdscs(config,
                                flatten_to_pdsc_future(vidx, &client, handle),
                                &client,
                                &mut core).collect();
    core.run(future).map(void).ok()
}
