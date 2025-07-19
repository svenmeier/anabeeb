use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use fluidsynth::synth::Synth;
use log::{debug, error, info, warn};
use crate::disposition::midi::MidiMessage;

const FLUID_PANIC: i32 = -2;
const FLUID_ERR: i32 = -1;
const FLUID_WARN: i32 = 0;
const FLUID_INFO: i32 = 1;
const FLUID_DBG: i32 = 2;

pub const NOTE_ON: u8 = 0x90;
pub const NOTE_OFF: u8 = 0x80;
pub const PROGRAM_CHANGE: u8 = 0xC0;
pub const CONTROL_CHANGE: u8 = 0xB0;
pub const PITCH_BEND: u8 = 0xE0;

pub fn send(synth: &mut Synth, channel: u8, message: &MidiMessage) {
    let status = message[0];
    let data1 = message[1] as i32;
    let data2 = message[2] as i32;
    match status {
        NOTE_ON => {
            synth.noteon(channel as i32, data1, data2);
        },
        NOTE_OFF => {
            synth.noteoff(channel as i32, data1);
        },
        PROGRAM_CHANGE => {
            synth.program_change(channel as i32, data1);
        },
        CONTROL_CHANGE => {
            synth.cc(channel as i32, data1, data2);
        },
        PITCH_BEND => {
            synth.pitch_bend(channel as i32, (data2 * 128) + data1);
        },
        _ => {},
    }
}

pub fn synth_init_logging() {
    unsafe {
        fluid_set_log_function(FLUID_PANIC, Some(log_to_env_logger), std::ptr::null_mut());
        fluid_set_log_function(FLUID_ERR, Some(log_to_env_logger), std::ptr::null_mut());
        fluid_set_log_function(FLUID_WARN, Some(log_to_env_logger), std::ptr::null_mut());
        fluid_set_log_function(FLUID_INFO, Some(log_to_env_logger), std::ptr::null_mut());
        fluid_set_log_function(FLUID_DBG, Some(log_to_env_logger), std::ptr::null_mut());
    }
}


#[link(name = "fluidsynth")]
unsafe extern "C" {
    fn fluid_set_log_function(
        level: i32,
        func: Option<unsafe extern "C" fn(i32, *const c_char, *mut c_void)>,
        data: *mut c_void,
    );
}

unsafe extern "C" fn log_to_env_logger(level: i32, message: *const c_char, _: *mut c_void) {
    if message.is_null() {
        return;
    }

    let msg = unsafe { CStr::from_ptr(message) }.to_string_lossy();

    match level {
        FLUID_WARN => warn!("FluidSynth: {}", msg),
        FLUID_INFO => info!("FluidSynth: {}", msg),
        FLUID_DBG => debug!("FluidSynth: {}", msg),
        _ => error!("FluidSynth: {}", msg),
    }
}