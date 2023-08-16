use quickjspp::console::Level;
use quickjspp::JsValue;

pub fn log_handler(level: Level, args: Vec<JsValue>) {
    let formatted_string = format_string(args);
    match level {
        Level::Debug => log::debug!("{}", formatted_string),
        Level::Info => log::info!("{}", formatted_string),
        Level::Warn => log::warn!("{}", formatted_string),
        Level::Error => log::error!("{}", formatted_string),
        Level::Trace => log::trace!("{}", formatted_string),
        Level::Log => log::log!(log::Level::Info, "{}", formatted_string),
    }
}

fn format_string(args: Vec<JsValue>) -> std::string::String {
    let mut template_string = "".to_string();
    let mut others = vec![];
    let mut i = 0;
    for arg in args {
        let raw_string = match arg {
            JsValue::Undefined => "undefined".to_string(),
            JsValue::Null => "null".to_string(),
            JsValue::Bool(v) => v.to_string(),
            JsValue::Int(i) => i.to_string(),
            JsValue::Float(i) => i.to_string(),
            JsValue::String(s) => s,
            JsValue::Array(_) => "[array Array]".to_string(),
            JsValue::Object(_) => "[object Object]".to_string(),
            JsValue::Function(_) => "[function Function]".to_string(),
            JsValue::Date(t) => t.to_rfc3339(),
            _ => todo!(),
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
            precision.push_str(".");
        } else if symbol_scan && precision_scan && character >= '0' && character <= '9' {
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
        formatted_string.push_str(" ");
        formatted_string.push_str(others.get(arg_pos).unwrap());
        arg_pos += 1;
    }

    return formatted_string;
}
