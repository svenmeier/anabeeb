use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use crate::disposition::general::{Disposition};

#[macro_export]
macro_rules! print_info {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        log::info!("{}", msg);
        print!("{}\r\n", msg);
    }}
}

#[macro_export]
macro_rules! print_error {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        log::error!("{}", msg);
        print!("\x1b[91m{}\x1b[0m\r\n", msg);
    }}
}

pub fn read_disposition(path: &String) -> Result<Disposition, Box<dyn Error>> {
    let json = &fs::read_to_string(path)?;

    let mut disposition: Disposition = serde_json::from_str(json)?;

    disposition._path = Some(path.clone());
    
    Ok(disposition)
}

pub fn write_disposition(disposition: &Disposition) -> Result<(), Box<dyn Error>> {
    let json  = serde_json::to_string_pretty(disposition)?;

    let path = disposition._path.clone().ok_or("no file specified")?;

    File::create(path)?.write_all(json.as_bytes())?;

    Ok(())
}

pub fn combine_paths(base: &str, relative: &str) -> String {
    let base_path = Path::new(base);
    let relative_path = Path::new(relative);

    let combined = match base_path.parent() {
        Some(parent) => parent.join(relative_path),
        None => PathBuf::from(relative_path),
    };

    combined.to_string_lossy().into_owned()
}