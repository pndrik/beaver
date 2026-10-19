// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use boa_engine::{
    Context, JsError, JsResult, JsValue, Module, Source, builtins::promise::PromiseState,
    js_string, module::ModuleLoader,
};
use boa_runtime::Console;
use std::rc::Rc;

use super::{
    Engine, base64::register_base64, console::CapturedConsole, fetch::FilteredFetcher,
    fs::register_fs, module_loader::ChrootModuleLoader,
};
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
};

impl Engine {
    pub(super) fn get_context<M: ModuleLoader + 'static>(
        &self,
        ctx: &AppContext,
        loader: Rc<M>,
    ) -> Result<Context, AppError> {
        let failed = |what: &str, e: JsError| {
            app_error!(
                Internal,
                "tool_failed",
                &format!("Failed to {}: {}", what, e),
                ctx.clone()
            )
        };

        let mut context = Context::builder()
            .module_loader(loader)
            .build()
            .map_err(|e| failed("create JavaScript context", e))?;

        if let Some(limits) = self.run.limits {
            context
                .runtime_limits_mut()
                .set_loop_iteration_limit(limits.loop_iterations);
            context
                .runtime_limits_mut()
                .set_recursion_limit(limits.recursion);
        }

        Console::register_with_logger(CapturedConsole::new(self.console.clone()), &mut context)
            .map_err(|e| failed("register console", e))?;
        register_base64(&mut context).map_err(|e| failed("register base64", e))?;

        let scopes = &self.run.scopes;
        if !scopes.fetch.is_empty() {
            boa_runtime::register(
                (boa_runtime::extensions::FetchExtension(
                    FilteredFetcher::new(scopes.fetch.clone()),
                ),),
                None,
                &mut context,
            )
            .map_err(|e| failed("register fetch", e))?;
        }

        if !scopes.filesystem.is_empty() {
            register_fs(ctx, &self.chroot, scopes.filesystem.clone(), &mut context)
                .map_err(|e| failed("register fs", e))?;
        }

        Ok(context)
    }
}

pub(super) fn describe(context: &mut Context, error: JsError) -> String {
    match error.try_native(context) {
        Ok(native) => native.to_string(),
        Err(_) => error.to_string(),
    }
}

fn settle(context: &mut Context, value: JsValue) -> Result<JsValue, String> {
    context.run_jobs().map_err(|e| describe(context, e))?;

    let Some(promise) = value.as_promise() else {
        return Ok(value);
    };
    match promise.state() {
        PromiseState::Fulfilled(value) => Ok(value),
        PromiseState::Rejected(e) => Err(describe(context, JsError::from_opaque(e))),
        PromiseState::Pending => Err("Promise is still pending".to_string()),
    }
}

/// Loads and evaluates the module at `path` inside the root.
fn evaluate(
    context: &mut Context,
    loader: &ChrootModuleLoader,
    path: &str,
) -> Result<Module, String> {
    let module = loader
        .load(path, context)
        .map_err(|e| describe(context, e))?;
    let promise = module.load_link_evaluate(context);
    settle(context, promise.into())?;
    Ok(module)
}

fn export(context: &mut Context, module: &Module, name: &str) -> Result<JsValue, String> {
    module
        .namespace(context)
        .get(js_string!(name), context)
        .map_err(|e| describe(context, e))
}

fn call(
    context: &mut Context,
    function: &JsValue,
    arguments: &[JsValue],
) -> Result<JsValue, String> {
    let Some(function) = function.as_callable() else {
        return Err("TypeError: not a function".to_string());
    };
    let value = function
        .call(&JsValue::undefined(), arguments, context)
        .map_err(|e| describe(context, e))?;
    settle(context, value)
}

pub(super) fn run_inline(context: &mut Context, code: &str) -> Result<JsValue, String> {
    let wrapped = format!("(async () => {{\n{}\n}})()", code);
    let promise = context
        .eval(Source::from_bytes(wrapped.as_bytes()))
        .map_err(|e| describe(context, e))?;
    settle(context, promise)
}

/// The default export of the module at `path`, called and awaited if it is a function.
pub(super) fn run_default_export(
    context: &mut Context,
    loader: &ChrootModuleLoader,
    path: &str,
) -> Result<JsValue, String> {
    let module = evaluate(context, loader, path)?;
    let default = export(context, &module, "default")?;
    match default.is_callable() {
        true => call(context, &default, &[]),
        false => settle(context, default),
    }
}

pub(super) fn run_function(
    context: &mut Context,
    loader: &ChrootModuleLoader,
    path: &str,
    name: &str,
    arguments: &[serde_json::Value],
) -> Result<JsValue, String> {
    let module = evaluate(context, loader, path)?;
    let function = export(context, &module, name)?;
    if !function.is_callable() {
        return Err(format!("TypeError: export '{}' is not a function", name));
    }

    let arguments = arguments
        .iter()
        .map(|argument| JsValue::from_json(argument, context))
        .collect::<JsResult<Vec<JsValue>>>()
        .map_err(|e| describe(context, e))?;
    call(context, &function, &arguments)
}

pub(super) fn to_json(
    context: &mut Context,
    value: &JsValue,
) -> Result<Option<serde_json::Value>, String> {
    if value.is_undefined() {
        return Ok(None);
    }

    let json = context
        .global_object()
        .get(js_string!("JSON"), context)
        .map_err(|e| describe(context, e))?;
    let stringify = json
        .as_object()
        .map(|json| json.get(js_string!("stringify"), context))
        .transpose()
        .map_err(|e| describe(context, e))?
        .and_then(|stringify| stringify.as_callable())
        .ok_or_else(|| "JSON.stringify is not available".to_string())?;
    let serialized = stringify
        .call(&json, std::slice::from_ref(value), context)
        .map_err(|e| describe(context, e))?;

    match serialized.as_string() {
        Some(string) => serde_json::from_str(&string.to_std_string_escaped())
            .map(Some)
            .map_err(|e| format!("Failed to convert the result: {}", e)),
        None => Ok(None),
    }
}
