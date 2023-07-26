use std::collections::HashMap;
use std::ptr::null_mut;
use std::sync::Arc;
use std::time::Duration;

use hai_pal::env::entry_dir;
use hai_pal::task::{spawn_local, JoinHandle};
use quickjspp::{Context, ExecutionError, JsFunction};
use std::sync::Mutex;

use crate::console::log_handler;
use crate::module::{module_loader, module_normalize};

static mut TIMER_ID: i32 = 0;

pub struct QuickVM {
    context: Context,
    timer_handles: Arc<Mutex<HashMap<i32, JoinHandle<()>>>>,
}

unsafe impl Send for QuickVM {}
unsafe impl Sync for QuickVM {}

impl QuickVM {
    pub fn new() -> Self {
        let context = Context::builder()
            .console(log_handler)
            .build()
            .expect("Failed to create QuickJS context");

        context.set_module_loader(
            Box::new(module_loader),
            Some(Box::new(module_normalize)),
            null_mut(),
        );

        let timer_handles = Arc::new(Mutex::new(HashMap::new()));

        {
            let timer_handles = timer_handles.clone();
            context
                .add_callback("setTimeout", move |callback: JsFunction, duration: i32| {
                    let timer_id = unsafe {
                        TIMER_ID += 1;
                        TIMER_ID
                    };

                    let handle = {
                        let timer_handles = timer_handles.clone();
                        spawn_local(async move {
                            tokio::time::sleep(Duration::from_millis(duration as u64)).await;
                            callback.call(vec![]).unwrap();
                            timer_handles.lock().unwrap().remove(&timer_id);
                        })
                    };

                    timer_handles.lock().unwrap().insert(timer_id, handle);

                    return timer_id;
                })
                .unwrap();
        }

        {
            let timer_handles = timer_handles.clone();
            context
                .add_callback("setInterval", move |callback: JsFunction, duration: i32| {
                    let timer_id = unsafe {
                        TIMER_ID += 1;
                        TIMER_ID
                    };

                    let handle = spawn_local(async move {
                        loop {
                            tokio::time::sleep(Duration::from_millis(duration as u64)).await;
                            callback.call(vec![]).unwrap();
                        }
                    });

                    timer_handles.lock().unwrap().insert(timer_id, handle);

                    return timer_id;
                })
                .unwrap();
        }

        {
            let timer_handles = timer_handles.clone();
            let clear_timer = move |timer_id: i32| {
                if let Some(handle) = timer_handles.lock().unwrap().remove(&timer_id) {
                    handle.abort();
                }

                return 0;
            };

            // clearInterval is in fact the same as clearTimeout
            context
                .add_callback("clearTimeout", clear_timer.clone())
                .unwrap();
            context.add_callback("clearInterval", clear_timer).unwrap();
        }

        Self {
            context,
            timer_handles,
        }
    }

    pub fn context(&self) -> &Context {
        &self.context
    }

    pub fn prepare_entry(&self) -> Result<(), ExecutionError> {
        let module_name = {
            let entry_dir = entry_dir();

            if entry_dir.to_string().ends_with("/") {
                "./index".to_string()
            } else {
                let specifier = entry_dir.path_segments().unwrap().last().unwrap();
                format!("./{}", specifier)
            }
        };

        self.context.run_module(&module_name)
    }
}

impl Drop for QuickVM {
    fn drop(&mut self) {
        // abort all pending timers
        for (_, handle) in self.timer_handles.lock().unwrap().drain() {
            handle.abort();
        }
    }
}
