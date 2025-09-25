use std::io::IsTerminal;

use quickjs_rusty::console::Level;
use quickjs_rusty::OwnedJsValue;

pub fn log_handler(level: Level, args: Vec<OwnedJsValue>) {
    let formatted_string = format_string(args);
    match level {
        Level::Debug => log::debug!(target: "console", "{}", formatted_string),
        Level::Info => log::info!(target: "console", "{}", formatted_string),
        Level::Warn => log::warn!(target: "console", "{}", formatted_string),
        Level::Error => log::error!(target: "console", "{}", formatted_string),
        Level::Trace => log::trace!(target: "console", "{}", formatted_string),
        Level::Log => log::log!(target: "console", log::Level::Info, "{}", formatted_string),
    }
}

fn format_string(args: Vec<OwnedJsValue>) -> std::string::String {
    let mut template_string = "".to_string();
    let mut others = vec![];
    let mut i = 0;
    for arg in args {
        let raw_string = if std::io::stdout().is_terminal() {
            arg.to_json_string(0).unwrap()
        } else {
            arg.js_to_string().unwrap()
        };

        if i == 0 {
            template_string = raw_string;
        } else {
            others.push(raw_string);
        }
        i += 1;
    }
    let mut symbol_scan = false;
    let mut precision_scan = false;
    let mut arg_pos = 0;
    let mut precision = "".to_string();
    let mut formatted_string = "".to_string();
    // template_string.
    for character in template_string.chars() {
        if symbol_scan && !precision_scan && character == '.' {
            precision_scan = true;
            precision.push('.');
        } else if symbol_scan && precision_scan && character.is_ascii_digit() {
            precision.push(character);
        } else if symbol_scan {
            symbol_scan = false;
            precision_scan = false;

            if arg_pos < others.len() {
                match character {
                    's' | 'f' | 'd' | 'i' => {
                        formatted_string.push_str(others.get(arg_pos).unwrap());
                        arg_pos += 1;
                    }
                    _ => {
                        formatted_string.push('%');
                        formatted_string.push_str(precision.as_str());
                        formatted_string.push(character);
                    }
                }
            } else {
                formatted_string.push('%');
                formatted_string.push_str(precision.as_str());
                formatted_string.push(character);
            }
        } else if character == '%' {
            symbol_scan = true;
            precision.clear();
        } else {
            formatted_string.push(character);
        }
    }

    while arg_pos < others.len() {
        formatted_string.push(' ');
        formatted_string.push_str(others.get(arg_pos).unwrap());
        arg_pos += 1;
    }

    formatted_string
}
