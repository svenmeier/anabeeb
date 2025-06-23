use std::error::Error;
use std::fs::{create_dir_all, File};
use std::io::Write;
use schemars::schema_for;
use anabeeb::disposition::general::{Disposition, MemoryState};

#[test]
fn generate_disposition() -> Result<(), Box<dyn Error>> {
    let schema = schema_for!(Disposition);

    let json = serde_json::to_string_pretty(&schema)?;

    create_dir_all("schemas")?;
    let mut file = File::create("schemas/disposition.schema.json")?;

    file.write_all(json.as_bytes())?;

    Ok(())
}

#[test]
fn generate_memory() -> Result<(), Box<dyn Error>> {
    let schema = schema_for!(MemoryState);

    let json = serde_json::to_string_pretty(&schema)?;

    create_dir_all("schemas")?;
    let mut file = File::create("schemas/memory.schema.json")?;

    file.write_all(json.as_bytes())?;

    Ok(())
}