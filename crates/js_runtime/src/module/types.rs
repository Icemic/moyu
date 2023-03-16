use hai_pal::url::Url;
use v8::{Global, Module as V8Module, Value};

#[derive(Debug, Clone)]
pub enum ModuleType {
    // path to local disk
    Local,
    // url
    Remote,
}

#[derive(Debug, Clone)]
pub struct Module {
    pub specifier: std::string::String,
    pub resolved_file_path: Url,
    pub module_type: ModuleType,
    pub script_id: Option<i32>,
    pub module: Option<Global<V8Module>>,
    pub result: Option<Global<Value>>,
}
