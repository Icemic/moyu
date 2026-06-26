use anyhow::Result;

use crate::utils::convert::to_js;

#[cfg(all(native, feature = "js_runtime"))]
pub async fn eval_in_sandbox(code: String) -> Result<serde_json::Value> {
    use crate::utils::convert::from_js;
    use moyu_runtime::try_get_vm;

    if let Some(vm) = try_get_vm() {
        let (sender, receiver) = moyu_pal::sync::oneshot::channel();

        let dispatch = move |vm: &moyu_runtime::QuickVM| {
            let code = to_js(&code).unwrap();

            let ret = match vm.call_function_direct("__moyu_eval_sandbox", vec![code]) {
                Ok(result) => from_js(&result),
                Err(err) => {
                    log::error!("failed to evaluate code: {}", err);
                    Err(anyhow::anyhow!("failed to evaluate code: {}", err))
                }
            };

            let _ = sender.send(ret);
        };

        if vm.is_vm_thread() {
            dispatch(vm);
        } else {
            vm.on_vm_thread(move |vm| dispatch(vm));
        }

        receiver.await?
    } else {
        Err(anyhow::anyhow!(
            "__moyu_eval_sandbox function not found in the global scope"
        ))
    }
}

#[cfg(web)]
pub async fn eval_in_sandbox(code: String) -> Result<serde_json::Value> {
    use wasm_bindgen::JsCast;
    use web_sys::js_sys::Function;

    let window = web_sys::window().unwrap();

    if let Some(__moyu_eval_sandbox) = window.get("__moyu_eval_sandbox") {
        if __moyu_eval_sandbox.is_function() {
            let __moyu_eval_sandbox = __moyu_eval_sandbox.unchecked_ref::<Function>();
            let code = match to_js(&code) {
                Ok(code) => code,
                Err(err) => {
                    log::error!("failed to convert code to JS: {:?}", err);
                    return Err(anyhow::anyhow!("failed to convert code to JS: {:?}", err));
                }
            };

            return match __moyu_eval_sandbox.call1(&window, &code) {
                Ok(result) => {
                    let result = result.into_serde()?;
                    Ok(result)
                }
                Err(err) => {
                    log::error!("failed to evaluate code: {:?}", err);
                    Err(anyhow::anyhow!("failed to evaluate code: {:?}", err))
                }
            };
        }
    };

    Err(anyhow::anyhow!(
        "__moyu_eval_sandbox function not found in the global scope"
    ))
}
