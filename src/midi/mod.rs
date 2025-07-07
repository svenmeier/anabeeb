use std::error::Error;
use log::{info, warn};
use midir::{MidiInput, MidiOutput};

pub mod channel_pool;

pub fn set_midi_channel(message: &Vec<u8>, new_channel: u8) -> Vec<u8> {
    let mut new_msg = message.clone();
    if let Some(status) = new_msg.get_mut(0) {
        *status = (*status & 0xF0) | (new_channel & 0x0F);
    }
    new_msg
}

pub fn get_wildcard(message: &[u8], pattern: &[u8]) -> Option<(usize, u8)> {
    if message.len() != pattern.len() {
        return None; // Length mismatch
    }

    let mut wildcard_index = None;

    for (i, (&msg_byte, &pat_byte)) in message.iter().zip(pattern.iter()).enumerate() {
        if pat_byte == 255 {
            if wildcard_index.is_none() {
                wildcard_index = Some((i, msg_byte));
            }
        } else if msg_byte != pat_byte {
            return None; // Mismatch
        }
    }

    wildcard_index
}

pub fn set_wildcard(message: &[u8], value: u8) -> Vec<u8> {
    message
        .iter()
        .map(|byte| if *byte == 255 { value } else { *byte })
        .collect()
}


pub fn log_midi_ports() {
    if let Err(e) = try_log_midi_ports() {
        warn!("failed to log midi ports: {}", e);
    }
}

pub fn try_log_midi_ports() -> Result<(), Box<dyn Error>> {
    for port in get_input_ports() {
        info!("Input Port '{}'", port);
    }

    for port in get_output_ports() {
        info!("Output Port '{}'", port);
    }

    Ok(())
}

pub fn get_input_ports() -> Vec<String> {
    let midi_in = MidiInput::new("anabeeb").unwrap();
    
    midi_in.ports().iter().map(| port | {
        midi_in.port_name(&port).unwrap()
    }).collect()
}

pub fn get_output_ports() -> Vec<String> {
    let midi_out = MidiOutput::new("anabeeb").unwrap();

    midi_out.ports().iter().map(| port | {
        midi_out.port_name(&port).unwrap()
    }).collect()
}