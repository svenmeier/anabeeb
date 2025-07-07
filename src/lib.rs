use clap::Parser;

pub mod disposition;
pub mod processor;
pub mod io;
pub mod console;
pub mod fluidsynth;
pub mod rouille;
pub mod midi;
pub mod setup;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long, default_value = "false", help = "write log to console instead of anabeeb.log")]
    pub log_to_console: bool,

    #[arg(long, default_value = "false", help = "save disposition on exit")]
    pub save_on_exit: bool,

    #[arg(long, default_value = "false", help = "save to the disposition instead of override")]
    pub save_no_override: bool,

    #[arg(long, default_value = "false", help = "setup elements")]
    pub setup: bool,

    #[arg(default_value = "disposition.json", help= "the disposition")]
    pub disposition: String,
}
