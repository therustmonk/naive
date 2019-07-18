#![feature(async_await, async_closure)]

use failure::{err_msg, Error};
use futures::Stream as _;
use futures3::compat::{Future01CompatExt, Sink01CompatExt, Stream01CompatExt};
use futures3::{Sink, SinkExt, Stream, StreamExt};
use serde_json::Value;
use std::borrow::Borrow;
use std::pin::Pin;
use tokio_tungstenite::connect_async;
use tungstenite::error::Error as WsError;
use tungstenite::Message;
use url::Url;

pub struct WebSocket {
    sink: Pin<Box<dyn Sink<Message, Error = WsError> + Send>>,
    stream: Pin<Box<dyn Stream<Item = Result<Message, WsError>> + Send>>,
}

impl WebSocket {
    pub async fn connect(url: impl AsRef<str>) -> Result<Self, Error> {
        let url = Url::parse(url.as_ref())?;
        let (ws_stream, _) = connect_async(url).compat().await?;
        let (sink, stream) = ws_stream.split();
        let (sink, stream) = (sink.sink_compat(), stream.compat());
        let (sink, stream) = (Box::pin(sink), Box::pin(stream));
        Ok(Self { sink, stream })
    }

    pub async fn send(&mut self, value: impl Borrow<Value>) -> Result<(), Error> {
        let text = serde_json::to_string(value.borrow())?;
        let msg = Message::Text(text);
        self.sink.send(msg).await?;
        Ok(())
    }

    pub async fn recv(&mut self) -> Result<Value, Error> {
        loop {
            let msg = self.stream.next().await;
            let msg = msg.ok_or_else(|| err_msg("websocket stream ended"))??;
            match msg {
                Message::Text(text) => {
                    let value = serde_json::from_str(&text)?;
                    return Ok(value);
                }
                Message::Binary(data) => {
                    let value = serde_json::from_slice(&data)?;
                    return Ok(value);
                }
                Message::Ping(_) | Message::Pong(_) => {}
                Message::Close(_) => {
                    return Err(err_msg("wsbsocket closed"));
                }
            }
        }
    }
}
