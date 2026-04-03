use ::reqwest::{
    Client, Request, RequestBuilder,
    header::{ACCEPT, HeaderValue},
};

use super::{LAST_EVENT_ID_HEADER, SSE_CONTENT_TYPE, into_body_stream, validate_response};
use crate::{
    error::{Error, ErrorKind},
    replayable::{ConnectFuture, Connector},
};

pub(crate) fn freeze_request(builder: RequestBuilder) -> Result<ReplayableRequest, Error> {
    let (client, request) = builder.build_split();
    let mut prototype = request.map_err(|err| {
        Error::new(ErrorKind::InvalidRequest, "request build failed").set_source(err)
    })?;

    if !prototype.headers().contains_key(ACCEPT) {
        prototype
            .headers_mut()
            .insert(ACCEPT, HeaderValue::from_static(SSE_CONTENT_TYPE));
    }

    ReplayableRequest::new(client, prototype)
}

#[derive(Debug)]
pub(crate) struct ReplayableRequest {
    client: Client,
    prototype: Request,
}

impl Clone for ReplayableRequest {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            prototype: self
                .prototype
                .try_clone()
                .expect("replayable requests must remain cloneable"),
        }
    }
}

impl ReplayableRequest {
    fn new(client: Client, prototype: Request) -> Result<Self, Error> {
        if prototype.headers().contains_key(LAST_EVENT_ID_HEADER) {
            return Err(Error::new(ErrorKind::InvalidRequest, "last event id header is reserved"));
        }

        if prototype.try_clone().is_none() {
            return Err(Error::new(ErrorKind::InvalidRequest, "request body is not replayable"));
        }

        Ok(Self { client, prototype })
    }

    pub(crate) fn prepare(&self, last_event_id: Option<&str>) -> Result<Request, Error> {
        let mut request = self
            .prototype
            .try_clone()
            .ok_or(Error::new(ErrorKind::InvalidRequest, "request body is not replayable"))?;

        request.headers_mut().remove(LAST_EVENT_ID_HEADER);

        if let Some(last_event_id) = last_event_id.filter(|value| !value.is_empty()) {
            let value = HeaderValue::from_str(last_event_id).map_err(|_| {
                Error::new(ErrorKind::InvalidRequest, "invalid `Last-Event-ID` header value")
            })?;
            request.headers_mut().insert(LAST_EVENT_ID_HEADER, value);
        }

        Ok(request)
    }
}

impl Connector for ReplayableRequest {
    fn connect(&self, last_event_id: Option<&str>) -> Result<ConnectFuture, Error> {
        let request = self.prepare(last_event_id)?;
        let client = self.client.clone();

        Ok(Box::pin(async move {
            let response = client.execute(request).await.map_err(|err| {
                Error::new(ErrorKind::Transport, "request execution failed").set_source(err)
            })?;

            validate_response(&response)?;

            Ok(into_body_stream(response))
        }))
    }
}
