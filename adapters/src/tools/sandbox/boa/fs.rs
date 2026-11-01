// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use boa_engine::{
    Context, Finalize, JsNativeError, JsObject, JsResult, JsString, JsValue, NativeFunction, Trace,
    js_string, object::builtins::JsArray, property::Attribute,
};
use tokio::runtime::Handle;

use app_domains::{
    core::models::{
        AppContext, AppError,
        file::{self, Chroot},
    },
    tools::models::{SandboxFilesystemOperation, SandboxFilesystemScope, SandboxScopes},
};

type FsFunction = fn(&JsValue, &[JsValue], &Bindings, &mut Context) -> JsResult<JsValue>;

#[derive(Trace, Finalize)]
struct Bindings {
    #[unsafe_ignore_trace]
    ctx: AppContext,
    #[unsafe_ignore_trace]
    chroot: Chroot,
    #[unsafe_ignore_trace]
    handle: Handle,
    #[unsafe_ignore_trace]
    scopes: SandboxScopes,
}

impl Bindings {
    /// `path` resolved in the chroot, if the scopes allow `operation` on it.
    fn permit(&self, path: &str, operation: SandboxFilesystemOperation) -> JsResult<file::Path> {
        let resolved = self
            .handle
            .block_on(self.chroot.resolve(&self.ctx, path))
            .map_err(denied)?;
        if self.scopes.allows_file(resolved.as_str(), operation) {
            Ok(resolved)
        } else {
            Err(error(format!(
                "Path '{}' is not allowed to {}",
                path,
                format!("{operation:?}").to_lowercase()
            )))
        }
    }
}

fn error(message: String) -> boa_engine::JsError {
    JsNativeError::error().with_message(message).into()
}

fn denied(e: AppError) -> boa_engine::JsError {
    error(e.internal_message)
}

fn argument(args: &[JsValue], index: usize, context: &mut Context) -> JsResult<String> {
    match args.get(index) {
        Some(value) if !value.is_undefined() => {
            Ok(value.to_string(context)?.to_std_string_escaped())
        }
        _ => Err(JsNativeError::typ()
            .with_message(format!("Missing argument {}", index + 1))
            .into()),
    }
}

fn read_file(
    _: &JsValue,
    args: &[JsValue],
    root: &Bindings,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let path = argument(args, 0, ctx)?;
    let resolved = root.permit(&path, SandboxFilesystemOperation::Read)?;
    let content = root
        .handle
        .block_on(root.chroot.read_to_string(&root.ctx, resolved.as_str()))
        .map_err(|e| error(format!("Failed to read '{}': {}", path, e.internal_message)))?;
    Ok(JsString::from(content).into())
}

fn read_dir(
    _: &JsValue,
    args: &[JsValue],
    root: &Bindings,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let path = argument(args, 0, ctx)?;
    let resolved = root.permit(&path, SandboxFilesystemOperation::Read)?;
    let names = root
        .handle
        .block_on(root.chroot.read_dir(&root.ctx, resolved.as_str()))
        .map_err(|e| error(format!("Failed to list '{}': {}", path, e.internal_message)))?
        .into_iter()
        .map(|entry| {
            let name = entry.path.file_name().unwrap_or_default();
            let suffix = if entry.metadata.is_dir() { "/" } else { "" };
            JsString::from(format!("{name}{suffix}")).into()
        });

    Ok(JsArray::from_iter(names, ctx).into())
}

fn write(
    args: &[JsValue],
    root: &Bindings,
    ctx: &mut Context,
    mode: file::WriteMode,
) -> JsResult<JsValue> {
    let path = argument(args, 0, ctx)?;
    let content = argument(args, 1, ctx)?;
    let resolved = root.permit(&path, SandboxFilesystemOperation::Write)?;
    root.handle
        .block_on(
            root.chroot
                .write(&root.ctx, resolved.as_str(), content.as_bytes(), mode),
        )
        .map_err(|e| {
            error(format!(
                "Failed to write '{}': {}",
                path, e.internal_message
            ))
        })?;

    Ok(JsValue::undefined())
}

fn write_file(
    _: &JsValue,
    args: &[JsValue],
    root: &Bindings,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    write(args, root, ctx, file::WriteMode::Truncate)
}

fn append_file(
    _: &JsValue,
    args: &[JsValue],
    root: &Bindings,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    write(args, root, ctx, file::WriteMode::Append)
}

pub(super) fn register_fs(
    ctx: &AppContext,
    chroot: &Chroot,
    handle: &Handle,
    scopes: Vec<SandboxFilesystemScope>,
    context: &mut Context,
) -> JsResult<()> {
    let scopes = SandboxScopes {
        fetch: vec![],
        filesystem: scopes,
    };
    let fs = JsObject::with_object_proto(context.intrinsics());
    let functions: [(JsString, FsFunction); 4] = [
        (js_string!("readFile"), read_file),
        (js_string!("readDir"), read_dir),
        (js_string!("writeFile"), write_file),
        (js_string!("appendFile"), append_file),
    ];

    for (name, function) in functions {
        let function = NativeFunction::from_copy_closure_with_captures(
            function,
            Bindings {
                ctx: ctx.clone(),
                chroot: chroot.clone(),
                handle: handle.clone(),
                scopes: scopes.clone(),
            },
        )
        .to_js_function(context.realm());
        fs.set(name, function, true, context)?;
    }

    context.register_global_property(js_string!("fs"), fs, Attribute::all())
}
