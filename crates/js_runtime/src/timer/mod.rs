use futures::{stream::FuturesUnordered, task::AtomicWaker};
use std::collections::HashMap;
use tokio::{task::JoinHandle, time::*};
use v8::{Function, Global};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimerType {
    Timeout,
    Interval,
}

/// not loyally obey the spec https://html.spec.whatwg.org/multipage/timers-and-user-prompts.html#timers
pub struct TimerScheduler {
    handler_id: HandlerId,
    callbacks: HashMap<HandlerId, Global<Function>>,
    timer_info: HashMap<HandlerId, (TimerType, u64)>,
    pub pending: FuturesUnordered<JoinHandle<HandlerId>>,
    pub waker: AtomicWaker,
}

pub type HandlerId = i32;

impl TimerScheduler {
    pub fn new() -> Self {
        Self {
            handler_id: 0,
            callbacks: Default::default(),
            timer_info: Default::default(),
            pending: Default::default(),
            waker: Default::default(),
        }
    }

    pub fn get_next_handler_id(&mut self) -> HandlerId {
        self.handler_id += 1;
        self.handler_id
    }

    pub fn add_timer(
        &mut self,
        t: TimerType,
        callback: Global<Function>,
        duration_millis: u64,
    ) -> HandlerId {
        let handler_id = self.get_next_handler_id();
        self.timer_info.insert(handler_id, (t, duration_millis));
        self.callbacks.insert(handler_id, callback);

        // minimum 4ms
        let duration_millis = duration_millis.max(4);
        let duration = Duration::from_millis(duration_millis);

        let tick_fn = async move {
            spin_sleep::sleep(duration);
            handler_id
        };

        self.pending.push(tokio::spawn(tick_fn));
        self.waker.wake();

        handler_id
    }

    /// reset timer to let interval emitted again
    pub fn reset_timer(&mut self, handler_id: HandlerId) {
        if let Some((t, duration_millis)) = self.timer_info.get(&handler_id) {
            assert_eq!(*t, TimerType::Interval);

            let duration_millis = *duration_millis;
            let tick_fn = async move {
                sleep(Duration::from_millis(duration_millis)).await;
                handler_id
            };

            self.pending.push(tokio::spawn(tick_fn));
            self.waker.wake();
        }
    }

    // just remove it from callback map,
    // future will polled but no callback to execute.
    // TODO: perform cancel in the next tick
    pub fn cancel_timer(&mut self, handler_id: HandlerId) {
        self.timer_info.remove(&handler_id);
        self.callbacks.remove(&handler_id);
    }

    /// consume callback by handler_id.
    /// for timeout, callback will be removed from map
    pub fn consume_callback(&mut self, handler_id: HandlerId) -> Option<Global<Function>> {
        if let Some((t, _)) = self.timer_info.get(&handler_id) {
            // IMPORTANT: should manually keep timer_info and callbacks have identical keys
            let callback = self.callbacks.get(&handler_id).unwrap().clone();

            if *t == TimerType::Timeout {
                self.timer_info.remove(&handler_id);
                self.callbacks.remove(&handler_id);
            } else {
                self.reset_timer(handler_id);
            }

            return Some(callback);
        }
        None
    }
}
