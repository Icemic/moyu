//! Web transport: the page owns the connection, so it uses the browser `WebSocket`
//! directly and reconnects through `setTimeout`.

use std::cell::RefCell;
use std::time::Duration;

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::js_sys;
use web_sys::{MessageEvent, WebSocket};

use super::{DebugSession, EvalFailure, EvalOutcome, emit, handle_request, hello};

const RETRY_DELAY: Duration = Duration::from_secs(1);

thread_local! {
    /// Keeps the current connection and its callbacks alive; assigning a new
    /// connection drops the previous one.
    static CONNECTION: RefCell<Option<Connection>> = const { RefCell::new(None) };
}

struct Connection {
    socket: WebSocket,
    _opened: Closure<dyn FnMut()>,
    _received: Closure<dyn FnMut(MessageEvent)>,
    _closed: Closure<dyn FnMut()>,
}

pub(super) fn start(session: DebugSession) {
    connect(session);
}

fn connect(session: DebugSession) {
    let socket = match WebSocket::new(&session.url) {
        Ok(socket) => socket,
        Err(_) => {
            retry(session);
            return;
        }
    };

    let opened = {
        let socket = socket.clone();
        let session = session.clone();

        Closure::<dyn FnMut()>::new(move || {
            super::set_outgoing(Some(super::Outgoing::new(
                send_via(socket.clone()),
                send_via(socket.clone()),
            )));
            let _ = socket.send_with_str(&hello(&session));
        })
    };

    let received = {
        let session = session.clone();

        Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
            let Some(text) = event.data().as_string() else {
                return;
            };

            let session = session.clone();

            // Requests are answered off the callback, as on native, so that one slow
            // handler cannot stall the connection.
            moyu_pal::task::spawn(async move {
                if let Some(response) = handle_request(&text, &session).await {
                    emit(response);
                }
            });
        })
    };

    // A closed socket is either the host going away or the page being unloaded; both
    // cases are handled by reconnecting, which simply does nothing if the page is gone.
    let closed = {
        let session = session.clone();

        Closure::<dyn FnMut()>::new(move || {
            super::on_disconnect();
            retry(session.clone());
        })
    };

    socket.set_onopen(Some(opened.as_ref().unchecked_ref()));
    socket.set_onmessage(Some(received.as_ref().unchecked_ref()));
    socket.set_onclose(Some(closed.as_ref().unchecked_ref()));

    CONNECTION.with(|slot| {
        let previous = slot.borrow_mut().replace(Connection {
            socket,
            _opened: opened,
            _received: received,
            _closed: closed,
        });

        if let Some(previous) = previous {
            let _ = previous.socket.close();
        }
    });
}

/// Evaluate a snippet in the page context. The page runs on the main thread, so the
/// timeout cannot interrupt a snippet that never returns; it is honoured on native only.
pub(super) async fn eval(code: &str, _timeout: Duration) -> Result<EvalOutcome, EvalFailure> {
    match web_sys::js_sys::eval(code) {
        Ok(value) => Ok(EvalOutcome::Value {
            repr: display(&value),
            json: json_text(&value),
        }),
        Err(thrown) => Ok(EvalOutcome::Thrown {
            message: property(&thrown, "message").unwrap_or_else(|| display(&thrown)),
            stack: property(&thrown, "stack"),
        }),
    }
}

/// Run a closure on the page's main thread.
///
/// The page has a single thread, which is also where input is processed, so there is
/// nothing to marshal over.
pub(super) async fn on_main_thread<T>(f: impl FnOnce() -> T) -> Result<T, String> {
    Ok(f())
}

/// Display form of a value, equivalent to `String(value)`.
///
/// wasm-bindgen has no binding for the `ToString` coercion, so the global `String`
/// function is called instead.
fn display(value: &JsValue) -> String {
    string_function()
        .and_then(|function| function.call1(&JsValue::UNDEFINED, value).ok())
        .and_then(|text| text.as_string())
        .unwrap_or_else(|| "<unprintable>".to_string())
}

/// JSON text of a value, absent when it has no JSON form (`undefined`, functions,
/// symbols) or cannot be serialized (circular references, BigInt values).
fn json_text(value: &JsValue) -> Option<String> {
    js_sys::JSON::stringify(value)
        .ok()
        .and_then(|text| text.as_string())
}

/// Read a string property of a thrown value. Such a value is usually an `Error`, but
/// JavaScript allows throwing anything.
fn property(value: &JsValue, name: &str) -> Option<String> {
    js_sys::Reflect::get(value, &JsValue::from_str(name))
        .ok()
        .and_then(|value| value.as_string())
}

fn string_function() -> Option<js_sys::Function> {
    js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("String"))
        .ok()
        .and_then(|value| value.dyn_into().ok())
}

/// Hand a message to the socket. The browser buffers outgoing frames, and a closed
/// socket fails the call, in which case the message is simply gone.
fn send_via(socket: WebSocket) -> impl Fn(String) + Send + Sync {
    move |text: String| {
        let _ = socket.send_with_str(&text);
    }
}

fn retry(session: DebugSession) {
    moyu_pal::task::set_timeout(RETRY_DELAY, async move { connect(session) });
}
