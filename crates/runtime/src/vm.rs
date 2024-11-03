use std::collections::VecDeque;
use std::ffi::{c_int, c_void};
use std::ptr::null_mut;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use hai_pal::config::get_engine_config;
use quickjs_rusty::{
    Arguments, Context, ExecutionError, JSContext, JsFunction, OwnedJsPromise, OwnedJsValue,
    RawJSValue,
};
use std::sync::Mutex;
use tokio::sync::oneshot::{Receiver, Sender};

use crate::console::log_handler;
use crate::module::{module_loader, module_normalize};

static mut TIMER_ID: i32 = 0;

pub struct QuickVM {
    context: Context,
    /// emited timer ids to be executed in the next tick
    timer_tasks: Arc<Mutex<Vec<Rc<TimerTask>>>>,
    instant: Instant,
    call_tasks: Arc<Mutex<VecDeque<(String, Vec<OwnedJsValue>, Sender<()>)>>>,
    async_tasks: Arc<Mutex<VecDeque<Box<dyn FnOnce(&Self)>>>>,
    to_be_closed: bool,
}

unsafe impl Send for QuickVM {}
unsafe impl Sync for QuickVM {}

#[derive(Debug, Clone, PartialEq)]
pub struct TimerTask {
    kind: TimerTaskKind,
    timer_id: i32,
    duration: i32,
    duration_until: u32,
    callback: JsFunction,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimerTaskKind {
    Timeout,
    Interval,
}

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

        context
            .set_host_promise_rejection_tracker(Some(host_promise_rejection_tracker), null_mut());

        let timer_tasks = Arc::new(Mutex::new(Vec::new()));
        let instant = Instant::now();

        {
            let timer_tasks = timer_tasks.clone();
            context
                .add_callback("setTimeout", move |callback: JsFunction, duration: i32| {
                    let timer_id = unsafe {
                        TIMER_ID += 1;
                        TIMER_ID
                    };

                    let duration_until = duration as u32 + instant.elapsed().as_millis() as u32;

                    let task = Rc::new(TimerTask {
                        kind: TimerTaskKind::Timeout,
                        timer_id,
                        duration,
                        duration_until,
                        callback,
                    });

                    timer_tasks.lock().unwrap().push(task);

                    timer_id
                })
                .unwrap();
        }

        {
            let timer_tasks = timer_tasks.clone();
            context
                .add_callback("setInterval", move |callback: JsFunction, duration: i32| {
                    let timer_id = unsafe {
                        TIMER_ID += 1;
                        TIMER_ID
                    };

                    let duration_until = duration as u32 + instant.elapsed().as_millis() as u32;

                    let task = Rc::new(TimerTask {
                        kind: TimerTaskKind::Interval,
                        timer_id,
                        duration,
                        duration_until,
                        callback,
                    });

                    timer_tasks.lock().unwrap().push(task);

                    timer_id
                })
                .unwrap();
        }

        {
            let timer_tasks = timer_tasks.clone();
            let clear_timer = move |args: Arguments| {
                let args = args.into_vec();
                if args.len() < 1 {
                    return;
                }

                // Do not panic if the argument is not a number
                // On MDN, https://developer.mozilla.org/en-US/docs/Web/API/clearTimeout
                // "Passing an invalid ID to clearTimeout() silently does nothing; no exception is thrown. "
                if let Ok(timer_id) = i32::try_from(args.get(0).cloned().unwrap()) {
                    let mut timer_tasks = timer_tasks.lock().unwrap();
                    if let Some(index) = timer_tasks
                        .iter()
                        .position(|task| task.timer_id == timer_id)
                    {
                        timer_tasks.remove(index);
                    }
                }
            };

            // clearInterval is in fact the same as clearTimeout
            context
                .add_callback("clearTimeout", clear_timer.clone())
                .unwrap();
            context.add_callback("clearInterval", clear_timer).unwrap();
        }

        Self {
            context,
            timer_tasks,
            instant,
            call_tasks: Arc::new(Mutex::new(VecDeque::new())),
            async_tasks: Arc::new(Mutex::new(VecDeque::new())),
            to_be_closed: false,
        }
    }

    /// Get the context of the vm, make sure to lock the vm before calling this function.
    ///
    /// This function must be called in the same thread where the vm is created.
    #[inline]
    pub fn context(&self) -> &Context {
        debug_assert_eq!(
            std::thread::current().name(),
            Some("quickjs"),
            "Detected called from wrong thread."
        );

        &self.context
    }

    /// Execute a function in the context of the vm and in quickjs thread.
    pub fn with_context(&self, f: impl FnOnce(&Self) + 'static) {
        self.async_tasks.lock().unwrap().push_back(Box::new(f));
    }

    ///
    /// Call a function directly instead of pushing it to the task queue.
    ///
    /// This function must be called in the same thread where the vm is created.
    pub fn call_function_direct(
        &self,
        name: &str,
        args: Vec<OwnedJsValue>,
    ) -> Result<OwnedJsValue, ExecutionError> {
        self.context().call_function(name, args)
    }

    pub fn call_function(&self, name: &str, args: Vec<OwnedJsValue>) -> Receiver<()> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        self.call_tasks
            .lock()
            .unwrap()
            .push_back((name.to_string(), args, sender));

        receiver
    }

    pub fn prepare_entry(&self) -> Result<OwnedJsPromise, ExecutionError> {
        let promise = self
            .context()
            .run_module(&get_engine_config().entry_filename)?;
        Ok(promise)
    }

    /// Tick the VM, executing all pending timers
    pub fn block_on_ticking(&self) {
        loop {
            {
                if self.to_be_closed {
                    break;
                }
            }

            // like microtasks in js, execute all async tasks until the queue is empty
            loop {
                let mut async_tasks = self.async_tasks.lock().unwrap();
                if let Some(task) = async_tasks.pop_front() {
                    drop(async_tasks);
                    task(&self);
                } else {
                    break;
                }
            }

            // handle all pending calls
            let mut call_tasks = self.call_tasks.lock().unwrap();
            while let Some((name, args, sender)) = call_tasks.pop_front() {
                let _result = self.context().call_function(&name, args);
                sender.send(()).unwrap();
            }

            // drop the lock before executing the tasks to avoid deadlocks
            drop(call_tasks);

            self.context().execute_pending_job().unwrap();

            // filter out all tasks that are ready to be executed
            let timer_tasks = self.timer_tasks.lock().unwrap();
            let mut tasks_to_execute = timer_tasks
                .iter()
                .filter_map(|task| {
                    let matched = task.duration_until <= self.instant.elapsed().as_millis() as u32;

                    if matched {
                        Some(task.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            // drop the lock before executing the tasks to avoid deadlocks
            drop(timer_tasks);

            // execute the tasks
            for task in tasks_to_execute.drain(..) {
                task.callback.call(vec![]).unwrap();

                // remove the task from the list
                let mut timer_tasks = self.timer_tasks.lock().unwrap();
                if let Some(index) = timer_tasks
                    .iter()
                    .position(|value| value.timer_id == task.timer_id)
                {
                    timer_tasks.remove(index);
                }

                // if the task is an interval, add it to the list again
                if task.kind == TimerTaskKind::Interval {
                    let duration_until =
                        task.duration as u32 + self.instant.elapsed().as_millis() as u32;

                    timer_tasks.push(Rc::new(TimerTask {
                        kind: TimerTaskKind::Interval,
                        timer_id: task.timer_id,
                        duration: task.duration,
                        duration_until,
                        callback: task.callback.clone(),
                    }));
                }
            }

            spin_sleep::sleep(std::time::Duration::from_millis(1));
        }
    }
}

impl Drop for QuickVM {
    fn drop(&mut self) {
        // clear all timer tasks before dropping the vm
        // or memory will leak
        self.to_be_closed = true;
        self.timer_tasks.lock().unwrap().clear();
        self.call_tasks.lock().unwrap().clear();
        self.async_tasks.lock().unwrap().clear();
    }
}

unsafe extern "C" fn host_promise_rejection_tracker(
    ctx: *mut JSContext,
    _promise: RawJSValue,
    reason: RawJSValue,
    is_handled: c_int,
    _opaque: *mut c_void,
) {
    let reason = OwnedJsValue::own(ctx, &reason);
    log::error!(
        "Promise rejection: {:?}, handled: {}",
        reason.js_to_string(),
        is_handled
    );
}
