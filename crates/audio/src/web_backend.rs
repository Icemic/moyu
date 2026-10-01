use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{Result, anyhow};
use kira::backend::{Backend, Renderer};
use send_wrapper::SendWrapper;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{
    AudioBufferSourceNode, AudioContext, AudioContextState, AudioScheduledSourceNode, Event, Window,
};

const CHUNK_FRAMES: usize = 2048;
const INITIAL_LOOKAHEAD_SECS: f64 = 0.15;
const MAX_LOOKAHEAD_SECS: f64 = 0.5;
const START_MARGIN_SECS: f64 = 0.005;

/// Schedule Kira's PCM output ahead of the browser's audio clock.
/// JS objects remain on the creating thread; SendWrapper checks this at runtime.
pub struct WebAudioBackend {
    state: SendWrapper<State>,
}

struct State {
    scheduler: Rc<RefCell<Scheduler>>,
    window: Window,
    pump_callback: Option<Closure<dyn FnMut(Event)>>,
    gesture_callback: Option<Closure<dyn FnMut(Event)>>,
}

struct Scheduler {
    ctx: AudioContext,
    ended_callback: Option<js_sys::Function>,
    renderer: Option<Renderer>,
    sample_rate: u32,
    next_time: Option<f64>,
    lookahead: f64,
    samples: Vec<f32>,
    channel_samples: Vec<f32>,
    sources: Vec<(AudioBufferSourceNode, f64)>,
}

fn js_error(error: JsValue) -> anyhow::Error {
    anyhow!("WebAudio: {:?}", error)
}

impl Backend for WebAudioBackend {
    type Settings = ();
    type Error = anyhow::Error;

    fn setup(_: (), _: usize) -> Result<(Self, u32)> {
        let window = web_sys::window().ok_or_else(|| anyhow!("WebAudio requires a window"))?;
        let ctx = AudioContext::new().map_err(js_error)?;
        let sample_rate = ctx.sample_rate() as u32;
        let scheduler = Rc::new(RefCell::new(Scheduler {
            ctx,
            ended_callback: None,
            renderer: None,
            sample_rate,
            next_time: None,
            lookahead: INITIAL_LOOKAHEAD_SECS,
            samples: vec![0.0; CHUNK_FRAMES * 2],
            channel_samples: vec![0.0; CHUNK_FRAMES],
            sources: Vec::new(),
        }));
        Ok((
            Self {
                state: SendWrapper::new(State {
                    scheduler,
                    window,
                    pump_callback: None,
                    gesture_callback: None,
                }),
            },
            sample_rate,
        ))
    }

    fn start(&mut self, renderer: Renderer) -> Result<()> {
        let state = &mut *self.state;
        if state.pump_callback.is_some() {
            return Err(anyhow!("WebAudio backend already started"));
        }
        state.scheduler.borrow_mut().renderer = Some(renderer);

        let weak = Rc::downgrade(&state.scheduler);
        state.pump_callback = Some(Closure::wrap(Box::new(move |_: Event| {
            if let Some(scheduler) = weak.upgrade() {
                let mut scheduler = scheduler.borrow_mut();
                if let Err(error) = scheduler.pump() {
                    log::error!("WebAudio scheduling failed: {error}");
                }
            }
        }) as Box<dyn FnMut(Event)>));

        let weak = Rc::downgrade(&state.scheduler);
        state.gesture_callback = Some(Closure::wrap(Box::new(move |_: Event| {
            if let Some(scheduler) = weak.upgrade() {
                scheduler.borrow().resume();
            }
        }) as Box<dyn FnMut(Event)>));

        let pump: &js_sys::Function = state
            .pump_callback
            .as_ref()
            .unwrap()
            .as_ref()
            .unchecked_ref();
        state.scheduler.borrow_mut().ended_callback = Some(pump.clone());
        let scheduler = state.scheduler.borrow();
        scheduler.ctx.set_onstatechange(Some(pump));
        let gesture = state
            .gesture_callback
            .as_ref()
            .unwrap()
            .as_ref()
            .unchecked_ref();
        for event in ["pointerdown", "keydown"] {
            state
                .window
                .add_event_listener_with_callback(event, gesture)
                .map_err(js_error)?;
        }
        scheduler.resume();
        drop(scheduler);
        state.scheduler.borrow_mut().pump()?;
        Ok(())
    }
}

impl Scheduler {
    fn resume(&self) {
        if self.ctx.state() == AudioContextState::Suspended {
            match self.ctx.resume() {
                Ok(promise) => {
                    wasm_bindgen_futures::spawn_local(async move {
                        if let Err(error) = wasm_bindgen_futures::JsFuture::from(promise).await {
                            log::warn!("WebAudio resume failed: {:?}", error);
                        }
                    });
                }
                Err(error) => log::warn!("WebAudio resume failed: {:?}", error),
            }
        }
    }

    /// Refill the lookahead window without depending on the engine frame loop.
    fn pump(&mut self) -> Result<()> {
        if self.ctx.state() != AudioContextState::Running {
            return Ok(());
        }
        let now = self.ctx.current_time();
        let step = CHUNK_FRAMES as f64 / self.sample_rate as f64;
        self.sources.retain(|(source, end)| {
            if *end <= now {
                source
                    .unchecked_ref::<AudioScheduledSourceNode>()
                    .set_onended(None);
                let _ = source.disconnect();
                false
            } else {
                true
            }
        });

        if let Some(next) = self.next_time {
            // Grow before an underrun where possible. Never shrink the queue mid-session:
            // doing so would require dropping PCM or leaving a gap in continuous playback.
            if next - now < step * 0.5 {
                let target =
                    (self.lookahead + step + (now - next).max(0.0)).min(MAX_LOOKAHEAD_SECS);
                if target > self.lookahead {
                    self.lookahead = target;
                    log::debug!("WebAudio lookahead increased to {:.0} ms", target * 1000.0);
                }
            }
        }
        let mut next = self.next_time.unwrap_or(now + START_MARGIN_SECS);
        if next < now {
            log::debug!("WebAudio underrun: {:.1} ms", (now - next) * 1000.0);
            next = now + START_MARGIN_SECS;
        }
        let horizon = now + self.lookahead;
        while next < horizon {
            let buffer = self
                .ctx
                .create_buffer(2, CHUNK_FRAMES as u32, self.sample_rate as f32)
                .map_err(js_error)?;
            let source = self.ctx.create_buffer_source().map_err(js_error)?;
            source
                .connect_with_audio_node(&self.ctx.destination())
                .map_err(js_error)?;
            let renderer = self.renderer.as_mut().unwrap();
            renderer.on_start_processing();
            renderer.process(&mut self.samples, 2);
            for channel in 0..2 {
                for (frame, sample) in self.channel_samples.iter_mut().enumerate() {
                    *sample = self.samples[frame * 2 + channel];
                }
                buffer
                    .copy_to_channel(&self.channel_samples, channel as i32)
                    .map_err(js_error)?;
            }
            source.set_buffer(Some(&buffer));
            // The callback retains only a Weak reference to this scheduler.
            source
                .unchecked_ref::<AudioScheduledSourceNode>()
                .set_onended(self.ended_callback.as_ref());
            // Rendering itself may consume the remaining slack; never schedule in the past.
            next = next.max(self.ctx.current_time() + START_MARGIN_SECS);
            source.start_with_when(next).map_err(js_error)?;
            next += step;
            self.next_time = Some(next);
            self.sources.push((source, next));
        }
        Ok(())
    }
}

impl Drop for State {
    fn drop(&mut self) {
        if let Some(callback) = &self.gesture_callback {
            for event in ["pointerdown", "keydown"] {
                let _ = self
                    .window
                    .remove_event_listener_with_callback(event, callback.as_ref().unchecked_ref());
            }
        }
        let mut scheduler = self.scheduler.borrow_mut();
        scheduler.ctx.set_onstatechange(None);
        scheduler.ended_callback = None;
        for (source, _) in scheduler.sources.drain(..) {
            source
                .unchecked_ref::<AudioScheduledSourceNode>()
                .set_onended(None);
            let _ = source.unchecked_ref::<AudioScheduledSourceNode>().stop();
            let _ = source.disconnect();
        }
        let _ = scheduler.ctx.close();
    }
}
