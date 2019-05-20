use futures::prelude::Future;
use futures::{Async, Poll};
use hyper::{Error, Body, Client, Response, StatusCode, Uri};
use hyper::client::{Connect, FutureResponse};
use hyper::header::Location;
use slog::Logger;

pub(crate) struct RedirectingFuture<'a, C: Connect> {
    client: &'a Client<C, Body>,
    uri: Uri,
    logger: &'a Logger,
    //history: Vec<Uri>,
    cur_get: FutureResponse,
}

pub(crate) trait ClientRedirExt<C>
where
    C: Connect,
{
    fn redirectable<'a>(
        &'a self,
        uri: Uri,
        logger: &'a Logger,
    ) -> Box<RedirectingFuture<'a, C>>;
}

impl<C: Connect> ClientRedirExt<C> for Client<C, Body> {
    fn redirectable<'a>(
        &'a self,
        uri: Uri,
        logger: &'a Logger,
    ) -> Box<RedirectingFuture<'a, C>> {
        Box::new(RedirectingFuture{
            client: self,
            uri: uri.clone(),
            logger,
            //history: Vec::new(),
            cur_get: self.get(uri),
        })
    }
}

impl<'a, C: Connect> Future for RedirectingFuture<'a, C> {
    type Item=Response;
    type Error=Error;
    fn poll(&mut self) -> Poll<Response, Error>{
        loop {
            debug!(self.logger, "Starting GET of {}", self.uri);
            match self.cur_get.poll()? {
                Async::NotReady => return Ok(Async::NotReady),
                Async::Ready(res) =>
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
                        if new_uri.authority().is_none() {
                            if let Some(authority) = self.uri.authority() {
                                new_uri = format!("{}{}", authority, self.uri).parse()?
                            }
                        }
                        debug!(self.logger, "Redirecting from {} to {}", self.uri, new_uri);
                        self.uri = new_uri;
                        //self.history.push(new_uri.clone());
                        self.cur_get = self.client.get(self.uri.clone());
                    }
                    _ => {
                        return Ok(Async::Ready(res));
                    }
                }
            }
        }
    }
}
