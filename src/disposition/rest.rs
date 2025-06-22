use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use crossbeam_channel::{bounded, Sender};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::thread;
use log::{debug, error, info};
use regex::{Captures, Regex};
use rouille::{websocket, Request, Response};
use schemars::JsonSchema;
use crate::disposition::general::{activate, combination_trigger, Disposition, Element, Id};
use crate::disposition::general::Element::{Captor, Combination, Coupler, MidiRange, MidiAction};
use crate::disposition::midi_out::{midi_activate, midi_change};
use crate::disposition::rest::Command::{Activate, Deactivate, NoOp, Trigger};
use crate::processor::{Event, Processor};
use std::time::Duration;
use crate::rouille::Client;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct RestConsole {
    pub port: u16,

    #[serde(skip)]
    _clients: Option<Clients>,
}

pub enum Command {
    Activate,
    Deactivate,
    Change(u32),
    Trigger,
    NoOp
}

type Clients = Arc<Mutex<Vec<Client>>>;

pub fn rest_init(disposition: &mut Disposition, processor: &Processor) -> Result<(), Box<dyn Error>> {
    for (_, element) in &mut disposition.elements {
        match element {
            Element::RestConsole(console) => {
                let clients = Arc::new(Mutex::new(vec![]));

                Some(start_server(console.port, processor.events.clone(), clients.clone()));

                console._clients = Some(clients);
            },
            _ => {},
        };
    }

    Ok(())
}

fn start_server(port: u16, events: Sender<Event>, clients: Clients) {
    thread::spawn(move || {
        rouille::start_server(format!("0.0.0.0:{}", port), move |request| {
            let mut response = Response::empty_404();

            if request.method() == "OPTIONS" {
                response = handle_options();
            } else {
                let path = request.url();

                if path == "/ws" {
                    response = handle_ws(request, clients.clone());
                } else if request.method() == "POST" {
                    let post_id_command_value = Regex::new(r"^/([^/]+)/([^/]+)(/([^/]+))?$").unwrap();
                    if let Some(caps) = post_id_command_value.captures(&path) {
                        response = handle_post(&events, caps);
                    }
                } else if request.method() == "GET" {
                    let get_id = Regex::new(r"^/([^/]+)$").unwrap();
                    if let Some(caps) = get_id.captures(&path) {
                        response = handle_get(&events, caps);
                    }
                }
            }

            response
                .with_additional_header("Access-Control-Allow-Origin", "*")
                .with_additional_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
                .with_additional_header("Access-Control-Allow-Headers", "Content-Type")
        })
    });
}

fn handle_options() -> Response {
    Response::text("").with_status_code(200)
}

fn handle_ws(request: &Request, clients: Clients) -> Response {
    let (response, websocket_receiver) = match websocket::start(request, Some("anabeeb")) {
        Ok(pair) => pair,
        Err(e) => {
            error!("websocket error: {}", e);
            return Response::empty_400()
        },
    };

    let client = Client::new(websocket_receiver);
    info!("connected from websocket client '{}'", client.id);
    clients.lock().unwrap().push(client);

    response
}

fn handle_get(events: &Sender<Event>, caps: Captures) -> Response{
    let id: Id = caps[1].into();

    send_and_receive(&events, id, NoOp)
}

fn handle_post(events: &Sender<Event>, caps: Captures) -> Response {
    let id: Id = caps[1].into();

    let command = match &caps[2] {
        "activate" => Activate,
        "deactivate" => Deactivate,
        "trigger" => Trigger,
        "change" => caps.get(4)
            .and_then(|m| m.as_str().parse::<u32>().ok())
            .map(Command::Change)
            .unwrap_or(NoOp),
        _ => NoOp,
    };
    send_and_receive(&events, id, command)
}

fn send_and_receive(events: &Sender<Event>, id: Id, command: Command) -> Response {
    let (sender, receiver) = bounded(1);

    // send the event
    events.send(Event::Rest(id, command, sender)).unwrap();

    // ... and wait until receiving a response
    match receiver.recv_timeout(Duration::from_secs(5)) {
        Ok(Some(result)) => Response::text(result),
        Ok(None) => Response::text("Not found").with_status_code(404),
        Err(_) => Response::text("Timeout").with_status_code(504),
    }
}

pub fn rest_process(disposition: &mut Disposition, event: &Event) {
    if let Event::Rest(id, command, sender) = event {
        rest_write(disposition, &id, command);

        let read = rest_read(disposition, &id);
        sender.send(read).unwrap();
    }
}

fn rest_write(disposition: &mut Disposition, id: &Id, command: &Command) {
    match disposition.elements.get_mut(id) {
        Some(Coupler(_)) => {
            if let Activate = command {
                activate(disposition, id.clone(), true)
            }
            if let Deactivate = command {
                activate(disposition, id.clone(), false)
            }
        },
        Some(Captor(_)) => {
            if let Activate = command {
                activate(disposition, id.clone(), true)
            }
            if let Deactivate = command {
                activate(disposition, id.clone(), false)
            }
        },
        Some(Combination(_)) => {
            if let Trigger = command {
                combination_trigger(disposition, id.clone())
            }
        },
        Some(MidiAction(_)) => {
            if let Activate = command {
                midi_activate(disposition, id.clone(), true)
            }
            if let Deactivate = command {
                midi_activate(disposition, id.clone(), false)
            }
        },
        Some(MidiRange(_)) => {
            if let Command::Change(value) = command {
                midi_change(disposition, id.clone(), value.clone())
            }
        },
        _ => {},
    }
}


pub fn rest_element_modified(disposition: &Disposition, id: Id) {
    let read = match rest_read(disposition, &id) {
        Some(read) => read,
        None => return,
    };

    for (_, element) in &disposition.elements {
        if let Element::RestConsole(console) = element {
            if let Some(clients) = &console._clients {
                debug!("sending to websocket clients");
                clients.lock().unwrap().retain_mut(|client| {
                    match client.send_text(&read) {
                        Ok(_) => {
                            debug!("modification sent to websocket client '{}'", client.id);
                            true
                        },
                        Err(e) => {
                            info!("disconnected from websocket client '{}': {:?}", client.id, e);
                            false
                        },
                    }
                });
            }
        }
    }
}

fn rest_read(disposition: &Disposition, id: &Id) -> Option<String> {
    match disposition.elements.get(id) {
        Some(element) => {
            let mut map = BTreeMap::new();
            map.insert(id, element);
            Some(serde_json::to_string_pretty(&map).unwrap())
        },
        _ => None,
    }
}