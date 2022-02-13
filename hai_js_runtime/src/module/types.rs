use hai_module_compiler::ScriptType;
use v8::{Global, Module, Value};

#[derive(Debug, Clone)]
pub enum ModuleType {
    // path to local disk
    Local(ScriptType),
    // url
    Remote,
    // file not exists or other errors
    None,
}

#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub specifier: std::string::String,
    pub module_referrer: std::string::String,
    pub resolved_specifier: std::string::String,
    pub module_type: ModuleType,
    pub script_id: Option<i32>,
    pub module: Option<Global<Module>>,
    pub result: Option<Global<Value>>,
}
