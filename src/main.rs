use std::error::Error;
use std::fs::File;
use std::process::exit;
use clap::Parser;
use env_logger::{Builder, Target};
use log::{LevelFilter};
use anabeeb::io::read_disposition;
use anabeeb::{print_error, print_info, Args};
use anabeeb::processor::{Processor};

fn main() -> Result<(), Box<dyn Error>> {
    print_info!("Welcome to Anabeeb");

    let args = Args::parse();

    init_logging(args.log_to_console);

    let mut disposition = match read_disposition(&args.disposition) {
        Ok(disposition) => {
            print_info!("loaded disposition '{}'", disposition._path.clone().unwrap().as_str());
            disposition
        },
        Err(e) => {
            print_error!("failed to load disposition '{}'", e);
            exit(1);
        },
    };
    
    let mut processor = Processor::new(args);

    print_info!("initializing disposition");
    processor.init(&mut disposition);

    print_info!("processing disposition");
    processor.process(&mut disposition);

    Ok(())
}

fn init_logging(log_to_console: bool) {
    let mut builder = Builder::from_default_env();
    builder.target(Target::Stderr);

    if let Err(_) = std::env::var("RUST_LOG") {
        builder.filter_level(LevelFilter::Info);
    }
    if log_to_console {
        // needed for term raw mode
        builder.format_suffix("\r\n");
    } else {
        let log_file = File::create("anabeeb.log").expect("Failed to create log file");
        builder.target(Target::Pipe(Box::new(log_file)));
    }
    builder.init();
}