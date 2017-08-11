use futures::{Stream};
use futures::future::{ok, Executor, IntoFuture};
use futures::stream::{iter, empty, FuturesUnordered, Concat2};
use futures::unsync::mpsc::{self, channel, unbounded, Receiver, UnboundedReceiver};
use hyper::{self, Client, Response, Body};
use hyper::client::{Connect};
use tokio_core::reactor::{Core, Handle};
use tokio_io::io::write_all;
use tokio_io::AsyncWrite;
use tokio_file_unix;
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write};

use minidom;

use super::{PdscRef, Vidx, Pidx};
use ::parse::FromElem;

static PIDX_SUFFIX: &'static str = ".pidx";

error_chain!{
    links{
        MinidomErr(minidom::Error, minidom::ErrorKind);
    }
    foreign_links{
        SinkErr(mpsc::SendError<Option<String>>);
        SinkErr2(mpsc::SendError<PdscRef>);
        HttpErr(hyper::Error);
        UriErr(hyper::error::UriError);
        IOErr(io::Error);
    }
}

future_chain!{}

fn void<T>(_: T) -> () { () }

pub fn flatten_to_pdsc_future<C>
    (Vidx{vendor_index, pdsc_index, ..}: Vidx,
     client: &Client<C, Body>,
     core: Handle) -> Receiver<PdscRef>
    where C: Connect {
    let mut job = FuturesUnordered::new();
    let (sender, reciever) = channel(vendor_index.len());
    for Pidx{url, vendor, ..} in vendor_index {
        let urlname = format!("{}{}{}", url, vendor, PIDX_SUFFIX);
        match urlname.parse() {
            Ok(uri) => {
                let work = client.get(uri)
                    .map(Response::body)
                    .flatten_stream()
                    .concat2()
                    .map_err(Error::from)
                    .and_then(move |body| {
                        let string = String::from_utf8_lossy(body.as_ref())
                            .into_owned();
                        Vidx::from_string(string.as_str())
                            .map_err(Error::from)
                            .map(|next_vidx| {
                                next_vidx.pdsc_index
                                    .into_iter()
                                    .map(Ok::<_, Error>)
                            })
                            .or_else(|e|{
                                println!("Error: Could not parse index from {}: {}", vendor, e);
                                Ok(Vec::new()
                                   .into_iter()
                                   .map(Ok::<_, Error>))
                            })
                    });
                job.push(work)

            }
            Err(e) => {
                println!("{}", e)
            }
        }
    }
    let stream = iter(pdsc_index.into_iter().map(Ok::<_, Error>))
        .chain(job.map(iter).flatten());
    core.execute(stream.forward(sender).map(void).map_err(void)).unwrap();
    reciever
}

pub fn download_pdscs<F, C>(stream: F,
                            client: &Client<C, Body>,
                            core: &mut Core) -> UnboundedReceiver<Option<String>>
    where F: Stream<Item = PdscRef, Error = ()>,
          C: Connect{
    let handle = core.handle();
    let (sender, reciever) = unbounded();
    let mut job = FuturesUnordered::new();
    core.run(stream
             .and_then(move |PdscRef{url, vendor, name, version, ..}| {
                 let uri = format!("{}{}.{}.pdsc", url, vendor, name)
                     .parse()
                     .map_err(void)?;
                 let filename = format!("{}.{}.{}.pdsc", vendor, name, version);
                 Ok((uri, filename))
             })
             .map(|(uri, filename)| {
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
                          .or_else(|_| Ok::<_, Error>(None)));
                 Ok::<_, ()>(())
             })
             .collect()
             .map(void)
    );
    core.execute(job.forward(sender).map(void).map_err(void));
    reciever
}

pub fn flatten_to_pdsc(vidx: Vidx) -> Result<Vec<PdscRef>> {
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let mut toret = Vec::new();
    let client = Client::new(&handle);
    toret.extend(core.run(flatten_to_pdsc_future(vidx, &client, handle).collect()).unwrap());
    Ok(toret)
}

pub fn flatten_to_downloaded_pdscs(vidx: Vidx) -> Option<()> {
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let client = Client::new(&handle);
    let future = download_pdscs(flatten_to_pdsc_future(vidx, &client, handle), &client, &mut core).collect();
    core.run(future).map(void).ok()
}
