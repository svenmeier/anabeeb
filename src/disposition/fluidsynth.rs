use std::error::Error;
use fluidsynth::audio::AudioDriver;
use fluidsynth::synth::{Synth};
use log::{debug, error, info};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::disposition::general::{Disposition, Element, Id};
use crate::fluidsynth::{send, synth_init_logging};
use crate::io::combine_paths;
use crate::midi::channel_pool::ChannelPool;
use crate::processor::{Processor};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct FluidsynthSound {
    #[serde(default = "default_soundfont")]
    pub soundfont: String,

    #[serde(default)]
    pub bank_offset: i32,
    
    pub gain: f32,
    
    pub interpolate: i32,

    pub settings: FluidsynthSettings,

    #[serde(skip, default)]
    _channels: ChannelPool,

    #[serde(skip)]
    _synth: Option<(Synth, AudioDriver)>,
}

fn default_soundfont() -> String { "disposition.sf2".to_string()}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct FluidsynthSettings {
    synth_overflow_age: f64,
    synth_overflow_percussion: f64,
    synth_overflow_released: f64,
    synth_overflow_sustained: f64,
    synth_overflow_volume: f64,
    synth_sample_rate: f64,
    synth_cpu_cores: i32,
    synth_midi_channels: i32,
    synth_polyphony: i32,
    synth_reverb_active: i32,
    synth_reverb_damp: f64,
    synth_reverb_level: f64,
    synth_reverb_width: f64,
    synth_reverb_room_size: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    audio_driver: Option<String>,
    audio_periods: i32,
    audio_period_size: i32,
}

pub fn fluidsynth_init(disposition: &mut Disposition, _: &Processor) {

    synth_init_logging();

    let path = disposition._path.as_deref().unwrap_or(".");

    for (id, element) in &mut disposition.elements {
        match element {
            Element::FluidsynthSound(sound) => {
                match create_synth(id, path, sound) {
                    Err(e) => {
                        error!("failed to create synth: {}", e)
                    },
                    Ok((synth, audiodriver)) => {
                        sound._synth = Some((synth, audiodriver));
                        info!("created synth")
                    }
                }
            },
            _ => {},
        }
    }
}

fn create_synth(id: &Id, path: &str, sound: &FluidsynthSound) -> Result<(Synth, AudioDriver), Box<dyn Error>> {
    let mut settings = fluidsynth::settings::Settings::new();
    settings.setnum("synth.overflow.age", sound.settings.synth_overflow_age);
    settings.setnum("synth.overflow.percussion", sound.settings.synth_overflow_percussion);
    settings.setnum("synth.overflow.released", sound.settings.synth_overflow_released);
    settings.setnum("synth.overflow.sustained", sound.settings.synth_overflow_sustained);
    settings.setnum("synth.overflow.volume", sound.settings.synth_overflow_volume);
    settings.setnum("synth.sample-rate", sound.settings.synth_sample_rate);
    settings.setint("synth.cpu-cores", sound.settings.synth_cpu_cores);
    settings.setint("synth.midi-channels", sound.settings.synth_midi_channels);
    settings.setint("synth.polyphony", sound.settings.synth_polyphony);

    settings.setint("synth.reverb.active", sound.settings.synth_reverb_active);
    settings.setnum("synth.reverb.damp", sound.settings.synth_reverb_damp);
    settings.setnum("synth.reverb.level", sound.settings.synth_reverb_level);
    settings.setnum("synth.reverb.width", sound.settings.synth_reverb_width);
    settings.setnum("synth.reverb.room-size", sound.settings.synth_reverb_room_size);

    if let Some(driver) = &sound.settings.audio_driver {
        settings.setstr("audio.driver", driver.as_str());
    }
    settings.setint("audio.periods", sound.settings.audio_periods);
    settings.setint("audio.period-size", sound.settings.audio_period_size);

    let mut synth = Synth::new(&mut settings);

    let combined_path = combine_paths(path, sound.soundfont.as_str());
    let soundfont_id = match synth.sfload(combined_path.as_str(), 0) {
        Some(soundfont_id) => {
            info!("loaded soundfont {} from '{}'", id, combined_path);
            soundfont_id
        },
        None => return Err(format!("Failed to load SoundFont '{}'", combined_path).as_str().into()),
    };
    
    synth.set_interp_method(-1, sound.interpolate);
    synth.set_gain(sound.gain);
    synth.set_bank_offset(soundfont_id as i32, sound.bank_offset);

    let driver = AudioDriver::new(&mut settings, &mut synth);

    Ok((synth, driver))
}

pub fn fluidsynth_send_messages(disposition: &mut Disposition, id: Id, channel: String, release: bool, messages: Vec<Vec<u8>>) {
    match disposition.elements.get_mut(&id) {
        Some(Element::FluidsynthSound(sound)) => {
            if let Some((ref mut synth, _)) = sound._synth {
                let (channel_number, new) = sound._channels.acquire(channel.as_str());
                if channel_number < sound.settings.synth_midi_channels as u8 {
                    for message in messages {
                        debug!("fluidsynth sound {} send '{}' {} {:?}", id, channel, channel_number, message);
                        send(synth, channel_number, message);
                    }
                } else {
                    if new {
                        error!("no channel available in {} for '{}'", id, channel);
                    }
                }

                if release {
                    sound._channels.release(channel.as_str());
                }
            }
        },
        _ => {}
    };
}