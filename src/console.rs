use atty::Stream;
use crokey::crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use dialoguer::{Completion, Input};
use log::{debug, warn};

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

#[macro_export]
macro_rules! read_line {
    ($var:ident) => {
        let mut $var = String::new();
        std::io::stdin().read_line(&mut $var)?;
        let $var = $var.trim().to_string();
    };
}

pub fn raw_mode(enabled: bool) {
    if enabled {
        if atty::is(Stream::Stdin) {
            // might hang if no TTY
            match enable_raw_mode() {
                Ok(()) => {
                    debug!("enabled raw mode");
                },
                Err(e) => {
                    warn!("could not enable raw mode: {}", e)
                }
            }
        } else {
            warn!("no raw mode since no TTY");
        }
    } else {
        if let Err(e) = disable_raw_mode() {
            warn!("Failed to disable raw mode: {}", e);
        }
    }
}

pub fn read_choice(prompt: &str, choices: Vec<String>) -> String {
    let completer = ChoicesCompleter{ choices };

    let input: String = Input::new()
        .with_prompt(prompt)
        .completion_with(&completer)
        .allow_empty(true)
        .interact_text()
        .unwrap();
    
    input.trim().to_string()
}

struct ChoicesCompleter {
    choices: Vec<String>,
}
impl Completion for ChoicesCompleter {
    fn get(&self, input: &str) -> Option<String> {
        let matches = self.choices
            .iter()
            .filter(|choice| {
                choice.starts_with(input)
            })
            .map(|id| id.to_string())
            .collect::<Vec<String>>();

        if matches.len() == 1 {
            Some(format!("{} ", matches[0].clone()))
        } else {
            Some(longest_common_prefix(matches.as_slice()))
        }
    }
}

fn longest_common_prefix(strings: &[String]) -> String {
    if strings.is_empty() {
        return "".to_string();
    }

    let mut prefix = strings[0].as_str();

    for s in &strings[1..] {
        let mut i = 0;
        let max_len = prefix.len().min(s.len());

        while i < max_len && prefix.as_bytes()[i] == s.as_bytes()[i] {
            i += 1;
        }

        prefix = &prefix[..i];

        if prefix.is_empty() {
            break;
        }
    }

    prefix.to_string()
}