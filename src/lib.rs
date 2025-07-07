use clap::Parser;

pub mod disposition;
pub mod processor;
pub mod io;
pub mod fluidsynth;
pub mod rouille;
pub mod midi;
pub mod setup;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long, default_value = "false")]
    pub log_to_console: bool,

    #[arg(long, default_value = "false")]
    pub save_on_exit: bool,

    #[arg(long, default_value = "false")]
    pub save_no_override: bool,

    #[arg(long, default_value = "false")]
    pub setup: bool,

    #[arg(default_value = "disposition.json")]
    pub disposition: String,
}
