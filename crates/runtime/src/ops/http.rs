use anyhow::{Result, anyhow};
use quickjs_rusty::{JSContext, OwnedJsValue, RawJSValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct FetchResponse {
    pub status: u16,
    pub status_text: String,
    pub ok: bool,
    pub headers: HashMap<String, String>,
    pub bytes: Vec<u8>,
}

pub(super) fn moyu_fetch(
    context: *mut JSContext,
    args: &[RawJSValue],
) -> Result<Option<RawJSValue>> {
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
