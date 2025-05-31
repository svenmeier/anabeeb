pub mod midi_manager;
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