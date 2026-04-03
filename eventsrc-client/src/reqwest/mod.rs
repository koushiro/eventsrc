mod request;
mod response;

pub(crate) use request::freeze_request;
pub(crate) use response::{into_body_stream, validate_response};

use crate::{error::Error, oneshot, replayable};

const SSE_CONTENT_TYPE: &str = "text/event-stream";
const LAST_EVENT_ID_HEADER: &str = "Last-Event-ID";

impl oneshot::EventSourceExt for ::reqwest::Response {
    fn event_source(self) -> Result<oneshot::EventSource, Error> {
        validate_response(&self)?;
        Ok(oneshot::EventSource::new(into_body_stream(self)))
    }
}

impl replayable::EventSourceExt for ::reqwest::RequestBuilder {
    fn event_source(self) -> Result<replayable::EventSource, Error> {
        Ok(replayable::EventSource::new(freeze_request(self)?))
    }
}
