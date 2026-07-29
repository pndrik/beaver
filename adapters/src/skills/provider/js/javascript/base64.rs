use base64::prelude::*;
use boa_engine::{Context, JsNativeError, JsResult, JsString, JsValue, NativeFunction, js_string};

fn btoa(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let input = args
        .get(0)
        .ok_or_else(|| JsNativeError::typ().with_message("btoa requires one argument"))?;

    let input_str = input.to_string(context)?.to_std_string_escaped();
    let encoded = BASE64_STANDARD.encode(input_str.as_bytes());

    Ok(JsString::from(encoded.as_str()).into())
}

fn atob(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let input = args
        .get(0)
        .ok_or_else(|| JsNativeError::typ().with_message("atob requires one argument"))?;

    let input_str = input.to_string(context)?.to_std_string_escaped();
    let decoded_bytes = BASE64_STANDARD
        .decode(input_str.as_bytes())
        .map_err(|e| JsNativeError::range().with_message(format!("Invalid base64 input: {}", e)))?;

    Ok(JsString::from(String::from_utf8_lossy(&decoded_bytes).to_string()).into())
}

pub fn register_base64(context: &mut Context) -> JsResult<()> {
    context.register_global_builtin_callable(
        js_string!("btoa"),
        1,
        NativeFunction::from_fn_ptr(btoa),
    )?;
    context.register_global_builtin_callable(
        js_string!("atob"),
        1,
        NativeFunction::from_fn_ptr(atob),
    )?;
    Ok(())
}
