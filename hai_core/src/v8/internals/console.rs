use v8::{
    FunctionCallbackArguments, FunctionTemplate, HandleScope, Local, Object, ObjectTemplate,
    ReturnValue, String,
};

pub fn init(handle_scope: &mut HandleScope, global: &Local<Object>) {
    bind_object! {
        to global;
        of handle_scope;
        "console" => {
            "log" => log,
            "info" => info,
            "debug" => debug,
            "warn" => warn,
            "error" => error,
            "trace" => trace
        }
    }
}

fn format_string(scope: &mut HandleScope, args: FunctionCallbackArguments) -> std::string::String {
    let length = args.length();
    let mut template_string = "".to_string();
    let mut others = vec![];
    for i in 0..length {
        let arg = args.get(i).to_string(scope).unwrap();
        let raw_string = arg.to_rust_string_lossy(scope);
        if i == 0 {
            template_string = raw_string;
        } else {
            others.push(raw_string);
        }
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

fn log(scope: &mut HandleScope, args: FunctionCallbackArguments, _: ReturnValue) {
    let formatted_string = format_string(scope, args);
    log::log!(log::Level::Info, "{}", formatted_string);
}

fn info(scope: &mut HandleScope, args: FunctionCallbackArguments, _: ReturnValue) {
    let formatted_string = format_string(scope, args);
    log::log!(log::Level::Info, "{}", formatted_string);
}

fn debug(scope: &mut HandleScope, args: FunctionCallbackArguments, _: ReturnValue) {
    let formatted_string = format_string(scope, args);
    log::log!(log::Level::Debug, "{}", formatted_string);
}

fn warn(scope: &mut HandleScope, args: FunctionCallbackArguments, _: ReturnValue) {
    let formatted_string = format_string(scope, args);
    log::log!(log::Level::Warn, "{}", formatted_string);
}

fn error(scope: &mut HandleScope, args: FunctionCallbackArguments, _: ReturnValue) {
    let formatted_string = format_string(scope, args);
    log::log!(log::Level::Error, "{}", formatted_string);
}

fn trace(scope: &mut HandleScope, args: FunctionCallbackArguments, _: ReturnValue) {
    let formatted_string = format_string(scope, args);
    log::log!(log::Level::Trace, "{}", formatted_string);
}
