use std::collections::HashMap;

pub struct State {
    pub module_referrer_names: HashMap<i32, String>,
}

impl State {
    pub fn new() -> Self {
        State {
            module_referrer_names: Default::default(),
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
}
