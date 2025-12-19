use std::cell::Cell;
use std::collections::{HashMap, VecDeque};
use std::ffi::c_void;
use std::ptr::null_mut;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::Instant;

use moyu_pal::config::get_engine_config;
use quickjs_rusty::{
    Arguments, Context, ExecutionError, JSContext, JsFunction, OwnedJsPromise, OwnedJsValue,
    RawJSValue,
};
use std::sync::Mutex;
use tokio::sync::oneshot::{Receiver, Sender};

use crate::console::log_handler;
use crate::module::{module_loader, module_normalize};

static mut TIMER_ID: i32 = 0;
static PROMISE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static IS_VM_THREAD: Cell<bool> = Cell::new(false);
}

/// Promise resolution task
#[derive(Debug)]
pub enum PromiseTask {
    Resolve { id: u64, value: OwnedJsValue },
    Reject { id: u64, error: String },
}

/// Promise Resolvers registry entry
#[derive(Debug)]
pub struct PromiseResolvers {
    resolve: JsFunction,
    reject: JsFunction,
}

pub struct QuickVM {
    context: Context,
    /// emited timer ids to be executed in the next tick
    timer_tasks: Arc<Mutex<Vec<Rc<TimerTask>>>>,
    instant: Instant,
    call_tasks: Arc<Mutex<VecDeque<(String, Vec<OwnedJsValue>, Sender<()>)>>>,
    async_tasks: Arc<Mutex<VecDeque<Box<dyn FnOnce(&Self) + Send>>>>,
    /// Promise resolvers registry - only accessed in QuickJS thread
    promise_resolvers: Arc<Mutex<HashMap<u64, PromiseResolvers>>>,
    /// Promise resolution task queue
    promise_tasks: Arc<Mutex<VecDeque<PromiseTask>>>,
    to_be_closed: bool,
}

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

impl Default for QuickVM {
    fn default() -> Self {
        Self::new()
    }
}

impl QuickVM {
    pub fn new() -> Self {
        // Mark current thread as VM thread
        IS_VM_THREAD.with(|flag| flag.set(true));

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

        context
            .global()
            .unwrap()
            .set_property("window", context.global().unwrap().into_value())
            .unwrap();

        context
            .global()
            .unwrap()
            .set_property("self", context.global().unwrap().into_value())
            .unwrap();

        context
            .eval(include_str!("injections/location.js"), false)
            .unwrap();

        context
            .eval(include_str!("injections/addeventlistener.js"), false)
            .unwrap();

        crate::websocket::register_websocket_ops(&context);
        context
            .eval(include_str!("injections/websocket.js"), false)
            .unwrap();

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
                if args.is_empty() {
                    return;
                }

                // Do not panic if the argument is not a number
                // On MDN, https://developer.mozilla.org/en-US/docs/Web/API/clearTimeout
                // "Passing an invalid ID to clearTimeout() silently does nothing; no exception is thrown. "
                if let Ok(timer_id) = i32::try_from(args.first().cloned().unwrap()) {
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
            promise_resolvers: Arc::new(Mutex::new(HashMap::new())),
            promise_tasks: Arc::new(Mutex::new(VecDeque::new())),
            to_be_closed: false,
        }
    }

    /// Get the context of the vm, make sure to lock the vm before calling this function.
    ///
    /// This function must be called in the same thread where the vm is created.
    #[inline]
    pub fn context(&self) -> &Context {
        &self.context
    }

    /// Execute a function in the context of the vm and in quickjs thread.
    pub fn on_vm_thread(&self, f: impl FnOnce(&Self) + Send + 'static) {
        self.async_tasks.lock().unwrap().push_back(Box::new(f));
    }

    /// Check if in VM thread
    #[inline(always)]
    pub fn is_vm_thread(&self) -> bool {
        IS_VM_THREAD.with(|flag| flag.get())
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

    /// Generate new Promise ID
    fn generate_promise_id() -> u64 {
        use std::sync::atomic::Ordering;
        PROMISE_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    /// Register Promise resolvers (only called in QuickJS thread)
    fn register_promise_resolvers(&self, id: u64, resolve: JsFunction, reject: JsFunction) {
        let resolvers = PromiseResolvers { resolve, reject };
        self.promise_resolvers.lock().unwrap().insert(id, resolvers);
    }

    /// Safely reject Promise from async tasks
    fn reject_promise(&self, id: u64, error: String) {
        let task = PromiseTask::Reject { id, error };
        self.promise_tasks.lock().unwrap().push_back(task);
    }

    /// Resolve Promise with an already created JS value (called in QuickJS thread)
    fn resolve_promise_with_value(&self, id: u64, js_value: OwnedJsValue) {
        let task = PromiseTask::Resolve {
            id,
            value: js_value,
        };
        self.promise_tasks.lock().unwrap().push_back(task);
    }

    /// Create Promise with automatic thread mode selection
    /// VM thread: immediate synchronous creation (zero overhead)
    /// Cross-thread: queued creation via on_vm_thread (thread-safe)
    pub fn create_promise<F, V>(&self, future: F) -> anyhow::Result<OwnedJsValue>
    where
        F: core::future::Future<Output = Result<V, anyhow::Error>> + Send + 'static,
        V: serde::Serialize + Send + 'static,
    {
        let promise_id = Self::generate_promise_id();

        let promise = if self.is_vm_thread() {
            let (promise, resolve, reject) = OwnedJsPromise::with_resolvers(self.context())?;
            self.register_promise_resolvers(promise_id, resolve, reject);
            promise.into_value()
        } else {
            let (sender, receiver) = std::sync::mpsc::sync_channel(1);

            self.on_vm_thread(move |vm_ref| {
                match OwnedJsPromise::with_resolvers(vm_ref.context()) {
                    Ok((promise, resolve, reject)) => {
                        vm_ref.register_promise_resolvers(promise_id, resolve, reject);
                        let _ = sender.send(Ok(promise.into_value()));
                    }
                    Err(e) => {
                        let _ = sender.send(Err(e));
                    }
                }
            });

            receiver
                .recv()
                .map_err(|_| anyhow::anyhow!("Failed to receive Promise from VM thread"))?
                .map_err(|e| anyhow::anyhow!("Failed to create Promise: {:?}", e))?
        };

        moyu_pal::task::get_runtime_handle().spawn(async move {
            match future.await {
                Ok(value) => {
                    let vm = crate::get_vm();
                    vm.on_vm_thread(move |vm_ref| {
                        // Use serde to properly serialize the value in QuickJS thread
                        match quickjs_rusty::serde::to_js(
                            unsafe { vm_ref.context().context_raw() },
                            &value,
                        ) {
                            Ok(js_value) => {
                                vm_ref.resolve_promise_with_value(promise_id, js_value);
                            }
                            Err(e) => {
                                vm_ref.reject_promise(
                                    promise_id,
                                    format!("Serialization error: {:?}", e),
                                );
                            }
                        }
                    });
                }
                Err(err) => {
                    crate::get_vm().reject_promise(promise_id, err.to_string());
                }
            }
        });

        Ok(promise)
    }

    /// Process all pending Promise tasks (only called in QuickJS thread)
    fn process_promise_tasks(&self) {
        let mut promise_tasks = self.promise_tasks.lock().unwrap();
        let mut promise_resolvers = self.promise_resolvers.lock().unwrap();

        while let Some(task) = promise_tasks.pop_front() {
            match task {
                PromiseTask::Resolve { id, value } => {
                    if let Some(resolvers) = promise_resolvers.remove(&id) {
                        // Use the already created JS value directly
                        if let Err(e) = resolvers.resolve.call(vec![value]) {
                            log::error!("Failed to resolve promise {}: {:?}", id, e);
                        }
                    }
                }
                PromiseTask::Reject { id, error } => {
                    if let Some(resolvers) = promise_resolvers.remove(&id) {
                        match self.context().eval(
                            &format!("new Error('{}')", error.replace("'", "\\'")),
                            false,
                        ) {
                            Ok(error_val) => {
                                if let Err(e) = resolvers.reject.call(vec![error_val]) {
                                    log::error!("Failed to reject promise {}: {:?}", id, e);
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to create error for promise {}: {:?}", id, e);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn prepare_entry(&self) -> Result<(), ExecutionError> {
        let promise = self
            .context()
            .run_module(&get_engine_config().entry_filename)?;

        self.context().resolve_value(promise.into_value())?;

        Ok(())
    }

    /// Tick the VM, executing all pending timers
    pub fn block_on_ticking(&self) {
        loop {
            if self.tick() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    /// Tick the VM, executing all pending timers
    pub fn tick(&self) -> bool {
        if self.to_be_closed {
            return true;
        }

        // like microtasks in js, execute all async tasks until the queue is empty
        loop {
            let mut async_tasks = self.async_tasks.lock().unwrap();
            if let Some(task) = async_tasks.pop_front() {
                drop(async_tasks);
                task(self);
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

        // Process Promise tasks
        self.process_promise_tasks();

        self.context().execute_pending_job().unwrap();

        // filter out all tasks that are ready to be executed
        let timer_tasks = self.timer_tasks.lock().unwrap();
        let mut tasks_to_execute = timer_tasks
            .iter()
            .filter_map(|task| {
                let matched = task.duration_until <= self.instant.elapsed().as_millis() as u32;

                if matched { Some(task.clone()) } else { None }
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

        false
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
        self.promise_resolvers.lock().unwrap().clear();
        self.promise_tasks.lock().unwrap().clear();

        crate::websocket::cleanup_websockets(unsafe { self.context.context_raw() } as usize);
    }
}

unsafe extern "C" fn host_promise_rejection_tracker(
    ctx: *mut JSContext,
    _promise: RawJSValue,
    reason: RawJSValue,
    is_handled: bool,
    _opaque: *mut c_void,
) {
    let reason = OwnedJsValue::own(ctx, &reason);
    if !is_handled {
        log::error!(
            "Promise rejection: {}",
            reason
                .js_to_string()
                .unwrap_or("Unknown reason".to_string())
        );
    }
}
