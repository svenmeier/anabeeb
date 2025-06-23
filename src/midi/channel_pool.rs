use std::collections::{HashMap, HashSet};
use log::debug;

pub struct ChannelPool {
    channels: HashMap<String, u8>,
}
impl Default for ChannelPool {
    fn default() -> Self {
        ChannelPool::new()
    }
}
impl ChannelPool {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
        }
    }

    pub fn acquire(&mut self, id: &str) -> (u8, bool) {
        if let Some(&channel) = self.channels.get(id) {
            return (channel, false);
        }

        let used: HashSet<u8> = self.channels.values().cloned().collect();
        for channel in 0.. {
            if !used.contains(&channel) {
                debug!("midi {} acquired channel {}", id, channel);
                self.channels.insert(id.to_string(), channel);
                return (channel, true);
            }
        }

        (0, true)
    }

    pub fn release(&mut self, id: &str) {
        if let Some(channel) = self.channels.remove(id) {
            debug!("midi {} released channel {}", id, channel);
        }
    }
}
