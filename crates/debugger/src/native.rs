//! Native transport: a tokio task owns the connection and reconnects until the engine
//! exits. Requests are answered on their own tasks, so a slow one (an evaluation
//! waiting for the VM thread, for example) keeps the connection responsive.

use std::time::Duration;

use futures_util::{Sink, SinkExt, Stream, StreamExt};
use moyu_pal::time::sleep;
use moyu_runtime::quickjs_rusty::{Context, ExecutionError, OwnedJsValue};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Error as WsError;
use tokio_tungstenite::tungstenite::Message;

use super::{DebugSession, EvalFailure, EvalOutcome, emit, handle_request, hello};

const RETRY_INTERVAL: Duration = Duration::from_secs(1);
/// Log pushes waiting for the socket; entries beyond this are dropped for a slow host.
const PUSH_BACKLOG: usize = 256;

pub(super) fn start(session: DebugSession) {
    moyu_pal::task::spawn(async move {
        let mut reported_failure = false;

        loop {
            match connect_async(&session.url).await {
                Ok((stream, _)) => {
                    reported_failure = false;
                    log::info!("Debug bridge connected to {}", session.url);

                    serve(stream, &session).await;
                    super::on_disconnect();
                }
                Err(err) => {
                    // The host may simply not be listening yet; report that once instead of
                    // writing a log line per retry.
                    if !reported_failure {
                        reported_failure = true;
                        log::warn!("Debug bridge cannot reach {}: {err}", session.url);
                    }
                }
            }

            sleep(RETRY_INTERVAL).await;
        }
    });
}

async fn serve<S>(stream: S, session: &DebugSession)
where
    S: Stream<Item = Result<Message, WsError>> + Sink<Message, Error = WsError> + Send + Unpin,
{
    let (mut sink, mut source) = stream.split();

    // Two paths reach the socket: responses, which are queued without limit because the
    // host waits for each of them, and log pushes, which wait in a small queue and are
    // dropped once it is full so that a slow host cannot grow the engine's memory.
    let (send_tx, mut send_rx) = mpsc::unbounded_channel::<String>();
    let (push_tx, mut push_rx) = mpsc::channel::<String>(PUSH_BACKLOG);

    super::set_outgoing(Some(super::Outgoing::new(
        move |text| {
            let _ = send_tx.send(text);
        },
        move |text| {
            let _ = push_tx.try_send(text);
        },
    )));

    if sink.send(Message::Text(hello(session).into())).await.is_err() {
        return;
    }

    loop {
        tokio::select! {
            outgoing = send_rx.recv() => {
                let Some(text) = outgoing else {
                    break;
                };

                if sink.send(Message::Text(text.into())).await.is_err() {
                    break;
                }
            }
            pushed = push_rx.recv() => {
                if let Some(text) = pushed
                    && sink.send(Message::Text(text.into())).await.is_err()
                {
                    break;
                }
            }
            incoming = source.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        let session = session.clone();

                        moyu_pal::task::spawn(async move {
                            if let Some(response) = handle_request(text.as_str(), &session).await {
                                emit(response);
                            }
                        });
                    }
                    Some(Ok(_)) => {}
                    Some(Err(_)) | None => break,
                }
            }
        }
    }
}

/// Evaluate a snippet on the VM thread, where the JavaScript context lives.
pub(super) async fn eval(code: &str, timeout: Duration) -> Result<EvalOutcome, EvalFailure> {
    let vm = moyu_runtime::try_get_vm().ok_or_else(|| {
        EvalFailure::Unavailable("JavaScript context is not available".to_string())
    })?;

    let code = code.to_string();
    let (tx, rx) = tokio::sync::oneshot::channel();

    vm.on_vm_thread(move |vm| {
        let _ = tx.send(evaluate(vm.context(), &code));
    });

    // The VM runs on the render loop, so the wait is bounded here instead of stalling
    // the connection.
    match tokio::time::timeout(timeout, rx).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err(EvalFailure::Unavailable(
            "VM thread dropped the task".to_string(),
        )),
        Err(_) => Err(EvalFailure::Timeout(timeout)),
    }
}

/// Run the snippet and describe the result. Everything happens on the VM thread,
/// because values read from the context cannot leave it.
fn evaluate(context: &Context, code: &str) -> Result<EvalOutcome, EvalFailure> {
    match context.eval(code, false) {
        Ok(value) => Ok(EvalOutcome::Value {
            repr: display(&value),
            json: json_text(&value),
        }),
        // The runtime hands exceptions over in their string form, which keeps the
        // message but carries no stack.
        Err(ExecutionError::Exception(thrown)) => Ok(EvalOutcome::Thrown {
            message: display(&thrown),
            stack: None,
        }),
        Err(err) => Err(EvalFailure::Unavailable(err.to_string())),
    }
}

/// Display form of a value, equivalent to `String(value)`.
fn display(value: &OwnedJsValue) -> String {
    value
        .js_to_string()
        .unwrap_or_else(|_| "<unprintable>".to_string())
}

/// JSON text of a value, absent when it has no JSON form (`undefined`, functions,
/// symbols) or cannot be serialized (circular references, BigInt values).
fn json_text(value: &OwnedJsValue) -> Option<String> {
    value.to_json_string(0).ok()
}
