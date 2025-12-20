use anyhow::{Result, anyhow};
use quickjs_rusty::{JSContext, OwnedJsValue, RawJSValue};

use crate::get_vm;

pub(super) fn moyu_eval(
    context: *mut JSContext,
    args: &[RawJSValue],
) -> Result<Option<RawJSValue>> {
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
