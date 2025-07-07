use std::io::stdin;
use crate::disposition::general::Disposition;
use crate::disposition::general::Element::{MidiConsole, MidiKeyboard, MidiSound, RestConsole};
use crate::midi::{get_input_ports, get_output_ports};
use crate::{print_info};
use crate::processor::Processor;

pub fn setup(disposition: &mut Disposition, processor: &Processor) -> Result<(), Box<dyn std::error::Error>> {

    print_info!("setting up elements");
    for (id, element) in &mut disposition.elements {
        match element {
            MidiConsole(console) => {
                print_info!("Midi Console {} - input port", id);

                console.port = choose(console.port.clone(), get_input_ports())?;
            },
            MidiKeyboard(keyboard) => {
                print_info!("Midi Keyboard {} - input port", id);

                keyboard.port = choose(keyboard.port.clone(), get_input_ports())?;
            },
            MidiSound(sound) => {
                print_info!("Midi Sound {} - output port", id);

                sound.port = choose(sound.port.clone(), get_output_ports())?;
            },
            RestConsole(console) => {
                print_info!("Rest Console {} - port", id);

                console.port = number(console.port.clone())?;
            },
            _ => {},
        }
    }

    print_info!("do you want to save the setup? (Y/n)");
    let mut input = String::new();
    stdin().read_line(&mut input)?;
    if "n" != input.trim().to_lowercase().as_str() {
        processor.save(disposition);
    }
    
    Ok(())
}

fn number(current: Option<u16>) -> Result<Option<u16>, Box<dyn std::error::Error>> {
    print_info!("enter a number or leave empty to keep {}", format(&current));
    
    let mut input = String::new();
    stdin().read_line(&mut input)?;
    if input.trim().len() == 0 {
        return Ok(current);
    }
    Ok(Some(input.trim().parse::<u16>()?))
}

fn choose(current: Option<String>, options: Vec<String>) -> Result<Option<String>, Box<dyn std::error::Error>> {
    print_info!(" [0] - None");
    for (index, option) in options.iter().enumerate() {
        print_info!(" [{}] - {}", index + 1, option);
    }
    print_info!("choose a number or leave empty to keep {}", format(&current));

    let mut input = String::new();
    stdin().read_line(&mut input)?;
    if input.trim().len() == 0 {
        return Ok(current);
    }
    let index = input.trim().parse::<usize>()?;
    if index == 0 {
        return Ok(None);
    }
    if index > options.len() {
        return Err(format!("{} is not a valid option", index).into());
    }
    Ok(Some(options[index - 1].clone()))
}

fn format<T: ToString>(option: &Option<T>) -> String {
    match option {
        Some(t) => format!("'{}'", t.to_string()),
        None => "none".to_string(),
    }
}