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

use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
    inference::models::chroot,
};

pub(super) struct ChrootModuleLoader {
    ctx: AppContext,
    root: PathBuf,
    modules: GcRefCell<HashMap<PathBuf, Module>>,
}

fn error(message: String) -> JsError {
    JsNativeError::typ().with_message(message).into()
}

impl ChrootModuleLoader {
    pub fn new(ctx: &AppContext, root: &str) -> Result<Self, AppError> {
        let root = Path::new(root).canonicalize().map_err(|e| {
            app_error!(
                Validation,
                "access_violation",
                &format!("Invalid chroot '{}': {}", root, e),
                ctx.clone()
            )
        })?;

        Ok(Self {
            ctx: ctx.clone(),
            root,
            modules: GcRefCell::default(),
        })
    }

    pub(super) fn resolve(&self, specifier: &str, referrer: Option<&Path>) -> JsResult<String> {
        let path = if specifier.starts_with('/') {
            PathBuf::from(specifier)
        } else if specifier.starts_with("./") || specifier.starts_with("../") {
            let directory = match referrer {
                Some(referrer) => Path::new("/")
                    .join(referrer.strip_prefix(&self.root).map_err(|_| {
                        error(format!(
                            "Module '{}' is outside of chroot",
                            referrer.display()
                        ))
                    })?)
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| PathBuf::from("/")),
                None => PathBuf::from("/"),
            };
            directory.join(specifier)
        } else {
            return Err(error(format!(
                "Cannot import '{}', only paths inside the chroot can be imported",
                specifier
            )));
        };

        let mut normalized = PathBuf::from("/");
        for component in path.components() {
            match component {
                Component::Normal(part) => normalized.push(part),
                Component::ParentDir if !normalized.pop() => {
                    return Err(error(format!("Import '{}' escapes the chroot", specifier)));
                }
                _ => {}
            }
        }

        Ok(normalized.to_string_lossy().to_string())
    }

    pub(super) fn load(&self, path: &str, context: &mut Context) -> JsResult<Module> {
        let root = self.root.to_string_lossy();
        let host = PathBuf::from(
            chroot::resolve_path(&self.ctx, &root, path)
                .map_err(|e| error(format!("Cannot import '{}': {}", path, e.internal_message)))?,
        );
        if !host.is_file() {
            return Err(error(format!("Cannot import '{}': not a file", path)));
        }

        if let Some(module) = self.modules.borrow().get(&host) {
            return Ok(module.clone());
        }

        let source = Source::from_filepath(&host)
            .map_err(|e| error(format!("Cannot read module '{}': {}", path, e)))?;
        let module = Module::parse(source, None, context)?;
        self.modules.borrow_mut().insert(host, module.clone());
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
        let result = self
            .resolve(
                &request.specifier().to_std_string_escaped(),
                referrer.path(),
            )
            .and_then(|path| self.load(&path, &mut context.borrow_mut()));

        async { result }
    }
}
