use std::sync::mpsc::{Receiver};
use rouille::websocket::{SendError, Websocket};
use uuid::Uuid;

pub struct Client {
    pub id: Uuid,

    receiver: Receiver<Websocket>,

    websocket: Option<Websocket>,
}

impl Client {
    pub fn new(receiver: Receiver<Websocket>) -> Self {
        Self {
            id: Uuid::new_v4(),
            receiver,
            websocket: None,
        }
    }

    pub fn send_text(&mut self, text: &str) -> Result<(), SendError> {
        let websocket = match self.websocket {
            Some(ref mut websocket) => websocket,
            None => {
                let websocket = self.receiver.recv().map_err(|_| SendError::Closed)?;
                self.websocket.insert(websocket)
            },
        };

        websocket.send_text(text)
    }
}