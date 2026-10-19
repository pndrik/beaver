// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use boa_engine::{
    Context, Finalize, JsNativeError, JsObject, JsResult, JsString, JsValue, NativeFunction, Trace,
    js_string, object::builtins::JsArray, property::Attribute,
};
use std::{
    fs,
    io::Write as _,
    path::{Path, PathBuf},
};

use app_domains::{
    core::models::{AppContext, AppError},
    inference::models::chroot,
    tools::models::{SandboxFilesystemOperation, SandboxFilesystemScope, SandboxScopes},
};

type FsFunction = fn(&JsValue, &[JsValue], &Chroot, &mut Context) -> JsResult<JsValue>;

#[derive(Trace, Finalize)]
struct Chroot {
    #[unsafe_ignore_trace]
    ctx: AppContext,
    #[unsafe_ignore_trace]
    root: String,
    #[unsafe_ignore_trace]
    canonical: PathBuf,
    #[unsafe_ignore_trace]
    scopes: SandboxScopes,
}

impl Chroot {
    fn absolute(path: &str) -> JsResult<()> {
        match path.starts_with('/') {
            true => Ok(()),
            false => Err(error(format!("Path must be absolute: '{}'", path))),
        }
    }

    fn permit(
        &self,
        path: &str,
        host: String,
        operation: SandboxFilesystemOperation,
    ) -> JsResult<String> {
        let resolved = Path::new("/").join(
            Path::new(&host)
                .strip_prefix(&self.canonical)
                .map_err(|_| error(format!("Path '{}' is outside of chroot", path)))?,
        );
        match self
            .scopes
            .allows_file(&resolved.to_string_lossy(), operation)
        {
            true => Ok(host),
            false => Err(error(format!(
                "Path '{}' is not allowed to {}",
                path,
                format!("{:?}", operation).to_lowercase()
            ))),
        }
    }

    fn readable(&self, path: &str) -> JsResult<String> {
        Self::absolute(path)?;
        let host = chroot::resolve_path(&self.ctx, &self.root, path).map_err(denied)?;
        self.permit(path, host, SandboxFilesystemOperation::Read)
    }

    fn writable(&self, path: &str) -> JsResult<String> {
        Self::absolute(path)?;
        let host = chroot::resolve_new_path(&self.ctx, &self.root, path).map_err(denied)?;
        self.permit(path, host, SandboxFilesystemOperation::Write)
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

fn read_file(_: &JsValue, args: &[JsValue], root: &Chroot, ctx: &mut Context) -> JsResult<JsValue> {
    let path = argument(args, 0, ctx)?;
    let host = root.readable(&path)?;
    if !Path::new(&host).is_file() {
        return Err(error(format!("Path '{}' is not a file", path)));
    }

    let content = fs::read_to_string(&host)
        .map_err(|e| error(format!("Failed to read '{}': {}", path, e)))?;
    Ok(JsString::from(content).into())
}

fn read_dir(_: &JsValue, args: &[JsValue], root: &Chroot, ctx: &mut Context) -> JsResult<JsValue> {
    let path = argument(args, 0, ctx)?;
    let host = root.readable(&path)?;

    let mut names = fs::read_dir(&host)
        .map_err(|e| error(format!("Failed to list '{}': {}", path, e)))?
        .filter_map(|entry| entry.ok())
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            match entry.path().is_dir() {
                true => name + "/",
                false => name,
            }
        })
        .collect::<Vec<String>>();
    names.sort();

    Ok(JsArray::from_iter(
        names.into_iter().map(|name| JsString::from(name).into()),
        ctx,
    )
    .into())
}

fn write(args: &[JsValue], root: &Chroot, ctx: &mut Context, append: bool) -> JsResult<JsValue> {
    let path = argument(args, 0, ctx)?;
    let content = argument(args, 1, ctx)?;
    let host = root.writable(&path)?;

    fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(&host)
        .and_then(|mut file| file.write_all(content.as_bytes()))
        .map_err(|e| error(format!("Failed to write '{}': {}", path, e)))?;

    Ok(JsValue::undefined())
}

fn write_file(
    _: &JsValue,
    args: &[JsValue],
    root: &Chroot,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    write(args, root, ctx, false)
}

fn append_file(
    _: &JsValue,
    args: &[JsValue],
    root: &Chroot,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    write(args, root, ctx, true)
}

pub(super) fn register_fs(
    ctx: &AppContext,
    root: &str,
    scopes: Vec<SandboxFilesystemScope>,
    context: &mut Context,
) -> JsResult<()> {
    let canonical = Path::new(root)
        .canonicalize()
        .map_err(|e| error(format!("Invalid chroot '{}': {}", root, e)))?;
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
            Chroot {
                ctx: ctx.clone(),
                root: root.to_string(),
                canonical: canonical.clone(),
                scopes: scopes.clone(),
            },
        )
        .to_js_function(context.realm());
        fs.set(name, function, true, context)?;
    }

    context.register_global_property(js_string!("fs"), fs, Attribute::all())
}
