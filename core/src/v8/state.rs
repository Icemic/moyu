use std::collections::HashMap;
use v8::{Global, Module};

pub struct State {
    pub module_referrer_names: HashMap<i32, String>,
    pub module_map: HashMap<String, Global<Module>>,
}

impl State {
    pub fn new() -> Self {
        State {
            module_referrer_names: Default::default(),
            module_map: Default::default(),
        }
    }
    pub fn get_module_referrer_name(&self, script_id: i32) -> Option<String> {
        if let Some(v) = self.module_referrer_names.get(&script_id) {
            return Some(v.clone());
        }
        None
    }

    pub fn save_module_referrer_name(&mut self, script_id: i32, referrer_name: String) {
        self.module_referrer_names.insert(script_id, referrer_name);
    }

    pub fn get_module(&mut self, referrer_name: &String) -> Option<&Global<Module>> {
        self.module_map.get(referrer_name)
    }

    pub fn save_module(&mut self, referrer_name: &String, module: Global<Module>) {
        self.module_map.insert(referrer_name.clone(), module);
    }
}
