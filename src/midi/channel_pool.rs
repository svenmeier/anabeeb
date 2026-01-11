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

    pub fn acquire(&mut self, key: &str) -> (u8, bool) {
        if let Some(&channel) = self.channels.get(key) {
            return (channel, false);
        }

        let used: HashSet<u8> = self.channels.values().cloned().collect();
        for channel in 0.. {
            if !used.contains(&channel) {
                debug!("acquired '{}' channel {}", key, channel);
                self.channels.insert(key.to_string(), channel);
                return (channel, true);
            }
        }

        (0, true)
    }

    pub fn release(&mut self, key: &str) {
        if let Some(channel) = self.channels.remove(key) {
            debug!("released '{}' channel {}", key, channel);
        }
    }
}
