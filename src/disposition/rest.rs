use std::collections::{BTreeMap};
use serde::{Deserialize, Serialize};
use crossbeam_channel::{bounded};
use std::sync::{Arc, Mutex};
use std::thread;
use log::{debug, error, info};
use regex::{Captures, Regex};
use rouille::{websocket, Request, Response, Server};
use schemars::JsonSchema;
use crate::disposition::general::{Disposition, Element, Id};
use crate::processor::{Event, Events};
use std::time::Duration;
use crate::{print_error, print_info};
use crate::rouille::Client;

#[derive(Serialize)]
struct ResponseData<'a> {
    elements: BTreeMap<&'a Id, &'a Element>,
}
impl<'a> ResponseData<'a> {
    pub fn new() -> Self {
        Self { 
            elements: BTreeMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct RestConsole {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
}

pub struct RestHandler {
    events: Events,
    clients: Clients,
}
impl RestHandler {
    pub fn new(events: Events) -> Self {
        Self {
            events,
            clients: Arc::new(Mutex::new(vec![])),
        }
    }

    pub fn init(&mut self, disposition: &mut Disposition) {
        for (id, element) in &mut disposition.elements {
            match element {
                Element::RestConsole(console) => {
                    if let Some(port) = console.port {
                        Some(start_server(id.clone(), port, self.events.clone(), self.clients.clone()));
                    }
                },
                _ => {},
            };
        }
    }

    pub fn process(&mut self, disposition: &mut Disposition, event: &Event) {
        match event {
            Event::RestResponse(id, result) => {
                let data = to_response_data(disposition, &id);
                result.send(data).unwrap();
            }
            Event::Modified(id) => {
                let data = match to_response_data(disposition, &id) {
                    Some(read) => read,
                    None => return,
                };

                debug!("sending to websocket clients");
                self.clients.lock().unwrap().retain_mut(|client| {
                    match client.send_text(&data) {
                        Ok(_) => {
                            debug!("modification ${} sent to websocket client '{}'", id, client.id);
                            true
                        },
                        Err(e) => {
                            info!("disconnected from websocket client '{}': {:?}", client.id, e);
                            false
                        },
                    }
                });
            },
            _ => {}
        }
    }
}

type Clients = Arc<Mutex<Vec<Client>>>;

fn start_server(id: Id, port: u16, events: Events, clients: Clients) {
    thread::spawn(move || {
        let server = Server::new(format!("0.0.0.0:{}", port), move |request| {
            let mut response = Response::empty_404();

            if request.method() == "OPTIONS" {
                response = options();
            } else {
                let path = request.url();

                if path == "/ws" {
                    response = ws(request, clients.clone());
                } else if request.method() == "POST" {
                    let element_id_event_value = Regex::new(r"^/element/([^/]+)/([^/]+)(/([^/]+))?$").unwrap();
                    if let Some(caps) = element_id_event_value.captures(&path) {
                        response = post_element(&events, caps);
                    } else {
                        let binding_id_command = Regex::new(r"^/binding/([^/]+)/([^/]+)$").unwrap();
                        if let Some(caps) = binding_id_command.captures(&path) {
                            response = post_binding(&events, caps);
                        }
                    }
                } else if request.method() == "GET" {
                    let element_id = Regex::new(r"^/element/([^/]+)$").unwrap();
                    if let Some(caps) = element_id.captures(&path) {
                        response = get_element(&events, caps);
                    }
                }
            }

            response
                .with_additional_header("Access-Control-Allow-Origin", "*")
                .with_additional_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
                .with_additional_header("Access-Control-Allow-Headers", "Content-Type")
        });
        
        match server {
            Ok(server) => {
                print_info!("connected ${} to port {}", id, server.server_addr());
                server.run();
            },
            Err(e) => {
                print_error!("console ${} failed to connect: {}", id, e);
            },
        }
    });
}

fn options() -> Response {
    Response::text("").with_status_code(200)
}

fn ws(request: &Request, clients: Clients) -> Response {
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

fn post_binding(events: &Events, caps: Captures) -> Response {
    let id: Id = caps[1].into();

    match &caps[2] {
        "start" => {
            events.send(Event::BindingStart(id.clone()));
        },
        "end" => {
            events.send(Event::BindingEnd);
        }
        _ => {
            return Response::empty_400();
        },
    };
    Response::text("").with_status_code(200)
}

fn get_element(events: &Events, caps: Captures) -> Response{
    let id: Id = caps[1].into();

    respond(&events, id)
}

fn post_element(events: &Events, caps: Captures) -> Response {
    let id: Id = caps[1].into();

    let event = match &caps[2] {
        "activate" => Event::Activate(id.clone(), true),
        "deactivate" => Event::Activate(id.clone(), false),
        "trigger" => Event::Trigger(id.clone()),
        "change" => {
            let value = match caps.get(4).and_then(|m| m.as_str().parse::<u32>().ok()) {
                Some(v) => v,
                None => return Response::empty_400(),
            };
            Event::Change(id.clone(), value)
        },
        _ => {
            return Response::empty_400();
        },
    };
    events.send(event);
    
    respond(&events, id)
}

fn respond(events: &Events, id: Id) -> Response {
    let (sender, receiver) = bounded(1);

    events.send(Event::RestResponse(id, sender));

    match receiver.recv_timeout(Duration::from_secs(5)) {
        Ok(Some(result)) => Response::text(result),
        Ok(None) => Response::text("Not found").with_status_code(404),
        Err(_) => Response::text("Timeout").with_status_code(504),
    }
}

fn to_response_data(disposition: &Disposition, id: &Id) -> Option<String> {
    match disposition.elements.get(id) {
        Some(element) => {
            let mut data = ResponseData::new();
            data.elements.insert(id, element);

            Some(serde_json::to_string_pretty(&data).unwrap())
        },
        _ => None,
    }
}
