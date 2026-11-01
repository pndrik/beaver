// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use boa_engine::{
    Context, JsError, JsNativeError, JsResult, Module, Source,
    module::{ModuleLoader, ModuleRequest, Referrer},
};
use boa_gc::GcRefCell;
use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Component, Path, PathBuf},
    rc::Rc,
};
use tokio::runtime::Handle;

use app_domains::{
    app_error,
    core::models::{
        AppContext, AppError,
        file::{self, Chroot},
    },
};

pub(super) enum Modules {
    Chroot { chroot: Chroot, handle: Handle },
    Directory(PathBuf),
}

impl Modules {
    pub fn directory(ctx: &AppContext, directory: &str) -> Result<Self, AppError> {
        std::fs::canonicalize(directory)
            .map(Self::Directory)
            .map_err(|e| {
                app_error!(
                    Validation,
                    "access_violation",
                    &format!("Invalid module directory '{directory}': {e}"),
                    ctx.clone()
                )
            })
    }

    fn read(&self, ctx: &AppContext, path: &file::Path) -> Result<Vec<u8>, String> {
        match self {
            Self::Chroot { chroot, handle } => handle
                .block_on(chroot.read(ctx, path.as_str(), None))
                .map_err(|e| e.internal_message),
            Self::Directory(root) => {
                let host = root
                    .join(path.as_str().trim_start_matches('/'))
                    .canonicalize()
                    .map_err(|e| e.to_string())?;
                if !host.starts_with(root) {
                    return Err("outside of the module directory".to_string());
                }
                std::fs::read(host).map_err(|e| e.to_string())
            }
        }
    }
}

pub(super) struct ChrootModuleLoader {
    ctx: AppContext,
    modules: Modules,
    loaded: GcRefCell<HashMap<file::Path, Module>>,
}

fn error(message: String) -> JsError {
    JsNativeError::typ().with_message(message).into()
}

impl ChrootModuleLoader {
    pub fn new(ctx: &AppContext, modules: Modules) -> Self {
        Self {
            ctx: ctx.clone(),
            modules,
            loaded: GcRefCell::default(),
        }
    }

    /// The absolute path `specifier` refers to from the module at `referrer`.
    pub(super) fn resolve(specifier: &str, referrer: Option<&Path>) -> JsResult<String> {
        let path = if specifier.starts_with('/') {
            PathBuf::from(specifier)
        } else if specifier.starts_with("./") || specifier.starts_with("../") {
            let directory = match referrer {
                Some(referrer) if referrer.has_root() => referrer
                    .parent()
                    .map_or_else(|| PathBuf::from("/"), Path::to_path_buf),
                Some(referrer) => {
                    return Err(error(format!(
                        "Module '{}' is outside of chroot",
                        referrer.display()
                    )));
                }
                None => PathBuf::from("/"),
            };
            directory.join(specifier)
        } else {
            return Err(error(format!(
                "Cannot import '{specifier}', only paths inside the chroot can be imported"
            )));
        };

        let mut normalized = PathBuf::from("/");
        for component in path.components() {
            match component {
                Component::Normal(part) => normalized.push(part),
                Component::ParentDir if !normalized.pop() => {
                    return Err(error(format!("Import '{specifier}' escapes the chroot")));
                }
                _ => {}
            }
        }

        Ok(normalized.to_string_lossy().to_string())
    }

    pub(super) fn load(&self, path: &str, context: &mut Context) -> JsResult<Module> {
        let cannot = |message: String| error(format!("Cannot import '{path}': {message}"));
        let path = file::Path::parse(&self.ctx, path).map_err(|e| cannot(e.internal_message))?;
        if let Some(module) = self.loaded.borrow().get(&path) {
            return Ok(module.clone());
        }

        let content = self.modules.read(&self.ctx, &path).map_err(cannot)?;
        let source = Source::from_bytes(&content).with_path(Path::new(path.as_str()));
        let module = Module::parse(source, None, context)?;
        self.loaded.borrow_mut().insert(path, module.clone());
        Ok(module)
    }
}

impl ModuleLoader for ChrootModuleLoader {
    fn load_imported_module(
        self: Rc<Self>,
        referrer: Referrer,
        request: ModuleRequest,
        context: &RefCell<&mut Context>,
    ) -> impl Future<Output = JsResult<Module>> {
        let result = ChrootModuleLoader::resolve(
            &request.specifier().to_std_string_escaped(),
            referrer.path(),
        )
        .and_then(|path| self.load(&path, &mut context.borrow_mut()));

        async { result }
    }
}
