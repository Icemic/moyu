use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::atomic::{AtomicI32, Ordering};

use anyhow::{Result, anyhow};
use futures_util::{SinkExt, StreamExt};
use quickjs_rusty::serde::to_js;
use quickjs_rusty::{Context, JSContext, OwnedJsValue, RawJSValue, q};
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::CloseFrame, tungstenite::protocol::Message,
};

static NEXT_WS_ID: AtomicI32 = AtomicI32::new(1);

struct WsHandle {
    tx: mpsc::UnboundedSender<Message>,
    vm_id: usize,
}

static WS_HANDLES: LazyLock<Mutex<HashMap<i32, WsHandle>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn register_websocket_ops(context: &Context) {
    let ws_connect_func = context.create_custom_callback(ws_connect).unwrap();
    context
        .set_global("__moyu_ws_connect", ws_connect_func)
        .unwrap();
    let ws_send_func = context.create_custom_callback(ws_send).unwrap();
    context.set_global("__moyu_ws_send", ws_send_func).unwrap();
    let ws_close_func = context.create_custom_callback(ws_close).unwrap();
    context
        .set_global("__moyu_ws_close", ws_close_func)
        .unwrap();
}

pub fn cleanup_websockets(vm_id: usize) {
    let mut handles = WS_HANDLES.lock().unwrap();
    let to_remove: Vec<i32> = handles
        .iter()
        .filter(|(_, handle)| handle.vm_id == vm_id)
        .map(|(id, _)| *id)
        .collect();

    for id in to_remove {
        if let Some(handle) = handles.remove(&id) {
            let _ = handle.tx.send(Message::Close(Some(CloseFrame {
                code: 1001.into(),
                reason: "VM Destroyed".into(),
            })));
        }
    }
}

fn ws_connect(context: *mut JSContext, args: &[RawJSValue]) -> Result<Option<RawJSValue>> {
    if args.len() < 1 {
        return Err(anyhow!("ws_connect requires 1 argument"));
    }

    let url: String = OwnedJsValue::own(context, &args[0]).try_into()?;
    let vm_id = context as usize;

    let id = NEXT_WS_ID.fetch_add(1, Ordering::SeqCst);

    moyu_pal::task::get_runtime_handle().spawn(async move {
        match connect_async(&url).await {
            Ok((ws_stream, _)) => {
                let (mut write, mut read) = ws_stream.split();
                let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

                {
                    let mut handles = WS_HANDLES.lock().unwrap();
                    handles.insert(id, WsHandle { tx, vm_id });
                }

                // Dispatch open event
                dispatch_ws_event(id, "open", None);

                let mut read_task = moyu_pal::task::get_runtime_handle().spawn(async move {
                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                dispatch_ws_event(id, "message", Some(WsData::Text(text)));
                            }
                            Ok(Message::Binary(bin)) => {
                                dispatch_ws_event(id, "message", Some(WsData::Binary(bin)));
                            }
                            Ok(Message::Close(frame)) => {
                                let (code, reason) = frame
                                    .map(|f| (f.code.into(), f.reason.to_string()))
                                    .unwrap_or((1000, "".to_string()));
                                dispatch_ws_event(id, "close", Some(WsData::Close(code, reason)));
                                break;
                            }
                            Err(e) => {
                                log::error!("WebSocket read error (id={}): {:?}", id, e);
                                dispatch_ws_event(id, "error", None);
                                break;
                            }
                            _ => {}
                        }
                    }
                    // Clean up handle on close
                    WS_HANDLES.lock().unwrap().remove(&id);
                });

                let mut write_task = tokio::spawn(async move {
                    while let Some(msg) = rx.recv().await {
                        if let Err(e) = write.send(msg).await {
                            log::error!("WebSocket write error (id={}): {:?}", id, e);
                            break;
                        }
                    }
                });

                tokio::select! {
                    _ = &mut read_task => write_task.abort(),
                    _ = &mut write_task => read_task.abort(),
                }
            }
            Err(e) => {
                log::error!("WebSocket connection failed (url={}): {:?}", url, e);
                dispatch_ws_event(id, "error", None);
                // Also dispatch close if connection failed to ensure JS state is updated
                dispatch_ws_event(id, "close", Some(WsData::Close(1006, e.to_string())));
            }
        }
    });

    Ok(Some(unsafe { q::JS_NewNumber(context, id as f64) }))
}

// fn ws_send(id: u32, data: OwnedJsValue) -> bool {
fn ws_send(context: *mut JSContext, args: &[RawJSValue]) -> Result<Option<RawJSValue>> {
    if args.len() < 2 {
        return Err(anyhow!("ws_send requires 2 arguments"));
    }

    let args: Vec<_> = args
        .iter()
        .map(|arg| OwnedJsValue::own(context, arg))
        .collect();

    let id: i32 = args[0].clone().try_into()?;
    let data = &args[1];

    let msg = if data.is_string() {
        Message::Text(data.to_string().unwrap())
    } else if data.is_array_buffer() {
        // Handle ArrayBuffer
        unsafe {
            let ctx = data.context();
            let mut len = 0;
            let ptr = q::JS_GetArrayBuffer(ctx, &mut len, data.as_inner().clone());
            if ptr.is_null() {
                return Err(anyhow!("Failed to get ArrayBuffer data"));
            }
            let slice = std::slice::from_raw_parts(ptr, len as usize);
            Message::Binary(slice.to_vec())
        }
    } else {
        return Err(anyhow!("ws_send data must be a string or ArrayBuffer"));
    };

    if let Some(handle) = WS_HANDLES.lock().unwrap().get(&id) {
        handle.tx.send(msg)?;
        Ok(None)
    } else {
        Err(anyhow!("WebSocket handle not found for id {}", id))
    }
}

// fn ws_close(id: u32, code: Option<u16>, reason: Option<String>) {
fn ws_close(context: *mut JSContext, args: &[RawJSValue]) -> Result<Option<RawJSValue>> {
    if args.len() < 1 {
        return Err(anyhow!("ws_close requires at least 1 argument"));
    }

    let args: Vec<_> = args
        .iter()
        .map(|arg| OwnedJsValue::own(context, arg))
        .collect();

    let id = args[0].clone().try_into()?;
    let code = if args.len() >= 2 {
        Some(TryInto::<i32>::try_into(args[1].clone())? as u16)
    } else {
        None
    };
    let reason = if args.len() >= 3 {
        Some(args[2].to_string()?)
    } else {
        None
    };

    if let Some(handle) = WS_HANDLES.lock().unwrap().get(&id) {
        let _ = handle.tx.send(Message::Close(Some(CloseFrame {
            code: code.unwrap_or(1000).into(),
            reason: reason.unwrap_or_default().into(),
        })));
    }

    Ok(None)
}

enum WsData {
    Text(String),
    Binary(Vec<u8>),
    Close(u16, String),
}

fn dispatch_ws_event(id: i32, event_type: &'static str, data: Option<WsData>) {
    let vm = crate::get_vm();
    vm.on_vm_thread(move |vm_ref| {
        let ctx = vm_ref.context();
        let mut args = vec![
            to_js(unsafe { ctx.context_raw() }, &id).unwrap(),
            to_js(unsafe { ctx.context_raw() }, &event_type).unwrap(),
        ];

        if let Some(ws_data) = data {
            match ws_data {
                WsData::Text(t) => {
                    args.push(to_js(unsafe { ctx.context_raw() }, &t).unwrap());
                }
                WsData::Binary(b) => {
                    // Create ArrayBuffer in QuickJS
                    unsafe {
                        let raw_ctx = ctx.context_raw();
                        let val = q::JS_NewArrayBufferCopy(raw_ctx, b.as_ptr(), b.len() as _);
                        args.push(OwnedJsValue::own(raw_ctx, &val));
                    }
                }
                WsData::Close(code, reason) => {
                    args.push(to_js(unsafe { ctx.context_raw() }, &(code as i32)).unwrap());
                    args.push(to_js(unsafe { ctx.context_raw() }, &reason).unwrap());
                }
            }
        }

        if let Err(e) = ctx.call_function("__moyu_ws_dispatch", args) {
            log::error!("Failed to call __moyu_ws_dispatch: {:?}", e);
        }
    });
}
