use futures::{channel::mpsc::Sender, SinkExt, StreamExt};
use reqwasm::websocket::{futures::WebSocket, Message};

use wasm_bindgen_futures::spawn_local;
use yew_agent::Dispatched;

use crate::services::event_bus::{EventBus, Request};

pub struct WebsocketService {
    pub tx: Sender<String>,
}

impl WebsocketService {
    pub fn new() -> Self {
        let host = web_sys::window()
            .and_then(|w| w.location().hostname().ok())
            .filter(|h| !h.is_empty())
            .unwrap_or_else(|| "127.0.0.1".to_string());
        let ws_url = format!("ws://{}:8080", host);
        let ws = WebSocket::open(&ws_url).unwrap();

        let (mut write, mut read) = ws.split();

        let (in_tx, mut in_rx) = futures::channel::mpsc::channel::<String>(1000);
        let mut event_bus = EventBus::dispatcher();

        spawn_local(async move {
            while let Some(s) = in_rx.next().await {
                log::debug!("got event from channel! {}", s);
                if let Err(e) = write.send(Message::Text(s)).await {
                    log::error!("failed to send websocket message: {:?}", e);
                    break;
                }
            }
        });

        spawn_local(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(data)) => {
                        log::debug!("from websocket: {}", data);
                        event_bus.send(Request::EventBusMsg(data));
                    }
                    Ok(Message::Bytes(b)) => {
                        let decoded = std::str::from_utf8(&b);
                        if let Ok(val) = decoded {
                            log::debug!("from websocket: {}", val);
                            event_bus.send(Request::EventBusMsg(val.into()));
                        }
                    }
                    Err(e) => {
                        log::error!("ws: {:?}", e)
                    }
                }
            }
            log::debug!("WebSocket Closed");
        });

        Self { tx: in_tx }
    }
}
