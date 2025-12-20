use anyhow::{Result, anyhow};
use quickjs_rusty::{Context, JSContext, OwnedJsValue, RawJSValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::get_vm;

#[derive(Serialize, Deserialize)]
pub struct FetchResponse {
    pub status: u16,
    pub status_text: String,
    pub ok: bool,
    pub headers: HashMap<String, String>,
    pub bytes: Vec<u8>,
}

pub fn register_http_ops(context: &Context) {
    let fetch_func = context.create_custom_callback(moyu_fetch).unwrap();
    context.set_global("__moyu_fetch", fetch_func).unwrap();
    let eval_func = context.create_custom_callback(moyu_eval).unwrap();
    context.set_global("__moyu_eval", eval_func).unwrap();
}

fn moyu_eval(context: *mut JSContext, args: &[RawJSValue]) -> Result<Option<RawJSValue>> {
    if args.len() < 1 {
        return Err(anyhow!("eval requires 1 argument"));
    }

    let code: String = OwnedJsValue::own(context, &args[0]).try_into()?;

    get_vm()
        .context()
        .eval(&code, false)
        .map(|v| unsafe { v.extract() })
        .map(|v| Some(v))
        .map_err(|e| anyhow::anyhow!("Eval error: {:?}", e))
}

fn moyu_fetch(context: *mut JSContext, args: &[RawJSValue]) -> Result<Option<RawJSValue>> {
    if args.len() < 1 {
        return Err(anyhow!("fetch requires at least 1 argument"));
    }

    let url: String = OwnedJsValue::own(context, &args[0]).try_into()?;

    // Optional options argument
    let _options = if args.len() >= 2 {
        Some(OwnedJsValue::own(context, &args[1]))
    } else {
        None
    };

    let vm = crate::get_vm();

    // We use create_promise to handle the async nature of fetch
    let promise = match vm.create_promise(async move {
        let request = ehttp::Request::get(&url);

        match ehttp::fetch_async(request).await {
            Ok(res) => {
                let mut headers = HashMap::new();
                for (k, v) in res.headers.into_iter() {
                    headers.insert(k.to_lowercase(), v.clone());
                }

                Ok(FetchResponse {
                    status: res.status,
                    status_text: res.status_text,
                    ok: res.ok,
                    headers,
                    bytes: res.bytes,
                })
            }
            Err(err) => Err(anyhow!("Fetch error: {}", err)),
        }
    }) {
        Ok(v) => v,
        Err(err) => {
            return Err(anyhow!("Failed to create promise for fetch: {}", err));
        }
    };

    Ok(Some(unsafe { promise.extract() }))
}
