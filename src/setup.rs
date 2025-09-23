use crate::disposition::Element::{MidiConsole, MidiKeyboard, MidiSound, RestConsole};
use crate::midi::{get_input_ports, get_output_ports};
use crate::{print_info, read_line};
use crate::disposition::Disposition;
use crate::processor::Processor;

pub fn setup(disposition: &mut Disposition, processor: &Processor) -> Result<(), Box<dyn std::error::Error>> {

    print_info!("setting up elements");
    for (id, element) in &mut disposition.elements {
        match element {
            MidiConsole(console) => {
                print_info!("Midi Console ${} - input port", id);

                if let Ok(ports) = get_input_ports() {
                    console.port = choose(console.port.clone(), ports)?;
                }
            },
            MidiKeyboard(keyboard) => {
                print_info!("Midi Keyboard ${} - input port", id);

                if let Ok(ports) = get_input_ports() {
                    keyboard.port = choose(keyboard.port.clone(), ports)?;
                }
            },
            MidiSound(sound) => {
                print_info!("Midi Sound ${} - output port", id);

                if let Ok(ports) = get_output_ports() {
                    sound.port = choose(sound.port.clone(), ports)?;
                }
            },
            RestConsole(console) => {
                print_info!("Rest Console ${} - port", id);

                console.port = number(console.port.clone())?;
            },
            _ => {},
        }
    }

    print_info!("do you want to save the setup? (Y/n)");
    read_line!(input);
    if "n" != input.to_lowercase().as_str() {
        processor.save(disposition);
    }
    
    Ok(())
}

fn number(current: Option<u16>) -> Result<Option<u16>, Box<dyn std::error::Error>> {
    print_info!("enter a number or leave empty to keep {}", format(&current));
    read_line!(input);
    if input.len() == 0 {
        return Ok(current);
    }
    Ok(Some(input.parse::<u16>()?))
}

fn choose(current: Option<String>, options: Vec<String>) -> Result<Option<String>, Box<dyn std::error::Error>> {
    print_info!(" [0] - None");
    for (index, option) in options.iter().enumerate() {
        print_info!(" [{}] - {}", index + 1, option);
    }
    print_info!("choose a number or leave empty to keep {}", format(&current));

    read_line!(input);
    if input.len() == 0 {
        return Ok(current);
    }
    let index = input.parse::<usize>()?;
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