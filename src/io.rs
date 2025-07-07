use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use serde_json::Value;
use serde_json::Value::Object;
use crate::disposition::general::{Disposition, MemoryState};

pub fn write_disposition(disposition: &Disposition) -> Result<(), Box<dyn Error>> {
    let path = disposition._path.clone().ok_or("no file specified")?;

    let json = serde_json::to_value(disposition)?;
    
    let string = serde_json::to_string_pretty(&json)?;

    File::create(path)?.write_all(string.as_bytes())?;

    Ok(())
}

pub fn write_disposition_override(disposition: &Disposition) -> Result<(), Box<dyn Error>> {
    let path = disposition._path.clone().ok_or("no file specified")?;

    let old_json = read_value(&path)?;
    let new_json = serde_json::to_value(disposition)?;

    let diff_json = json_diff(&old_json, &new_json);

    let string = serde_json::to_string_pretty(&diff_json)?;

    let override_path = path_with_suffix(&path, ".override.json");
    File::create(override_path)?.write_all(string.as_bytes())?;

    Ok(())
}

pub fn read_disposition(path: &String) -> Result<Disposition, Box<dyn Error>> {
    let mut value = read_value(path)?;

    let override_path = path_with_suffix(path, ".override.json");
    if Path::new(&override_path).exists() {
        let override_value = read_value(&override_path)?;
        json_merge(&mut value, &override_value);
    }

    let mut disposition: Disposition = serde_json::from_value(value)?;

    disposition._path = Some(path.clone());
    
    Ok(disposition)
}

fn read_value(path: &str) -> Result<Value, Box<dyn Error>> {
    let json = &fs::read_to_string(path)?;

    Ok(serde_json::from_str(&json)?)
}

fn json_merge(base_val: &mut Value, override_val: &Value) {
    match (base_val, override_val) {
        (Object(base_map), Object(override_map)) => {
            for (k, v) in override_map {
                json_merge(base_map.entry(k).or_insert(Value::Null), v);
            }
        },
        (base_val, override_val) => {
            *base_val = override_val.clone();
        },
    }
}

fn json_diff(old: &Value, new: &Value) -> Value {
    match (old, new) {
        (Object(old_map), Object(new_map)) => {
            let mut diff = serde_json::Map::new();

            for (key, new_val) in new_map {
                match old_map.get(key) {
                    Some(old_val) => {
                        let d = json_diff(old_val, new_val);
                        if !d.is_null() {
                            diff.insert(key.clone(), d);
                        }
                    }
                    None => {
                        diff.insert(key.clone(), new_val.clone());
                    }
                }
            }

            if diff.is_empty() {
                Value::Null
            } else {
                Object(diff)
            }
        }

        (Value::Array(a_arr), Value::Array(b_arr)) => {
            if a_arr != b_arr {
                new.clone() // Return the full array if there's any change
            } else {
                Value::Null
            }
        }

        _ => {
            if old != new {
                new.clone()
            } else {
                Value::Null
            }
        }
    }
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

fn path_with_suffix(path: &str, suffix: &str) -> String {
    let original = Path::new(path);

    let parent = original.parent().unwrap_or_else(|| Path::new(""));
    let stem = original.file_stem().unwrap_or_default();

    let mut relative_file = PathBuf::from(parent);
    relative_file.push(format!("{}{}", stem.to_string_lossy(), suffix));

    relative_file.to_string_lossy().into_owned()
}

pub fn read_memory(path: String) -> Result<MemoryState, Box<dyn Error>> {
    if Path::new(&path).exists() {
        let value = read_value(&path)?;
        let state: MemoryState = serde_json::from_value(value)?;

        return Ok(state);
    }
    
    Ok(MemoryState{ title: None, schema: None, levels: Vec::new() })
}

pub fn write_memory(path: String, state: &MemoryState) -> Result<(), Box<dyn Error>> {
    let json  = serde_json::to_string_pretty(state)?;
    File::create(path)?.write_all(json.as_bytes())?;

    Ok(())
}
