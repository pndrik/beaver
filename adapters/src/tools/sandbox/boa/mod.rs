// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use std::{cell::RefCell, rc::Rc};
use tokio::runtime::Handle;

use app_domains::{
    app_error,
    core::models::{AppContext, AppError, file::Chroot},
    tools::{
        models::{SandboxOutput, SandboxRun, SandboxSource},
        traits::CodeSandbox,
    },
};

mod base64;
mod console;
mod fetch;
mod fs;
mod helper;
mod module_loader;

#[cfg(test)]
mod tests;

pub struct Boa;

impl Boa {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Boa {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CodeSandbox for Boa {
    async fn run(
        &self,
        ctx: &AppContext,
        chroot: &Chroot,
        run: SandboxRun,
    ) -> Result<SandboxOutput, AppError> {
        let ctx_clone = ctx.clone();
        let chroot = chroot.clone();
        let handle = Handle::current();
        tokio::task::spawn_blocking(move || Engine::new(chroot, handle, run).run(&ctx_clone))
            .await
            .map_err(|e| {
                app_error!(
                    Internal,
                    "tool_failed",
                    &format!("JavaScript execution failed: {e}"),
                    ctx.clone()
                )
            })?
    }
}

/// One run on its own Boa context. Boa isn't `Send`, so it lives on the
/// blocking thread.
struct Engine {
    chroot: Chroot,
    handle: Handle,
    run: SandboxRun,
    console: Rc<RefCell<String>>,
}

impl Engine {
    fn new(chroot: Chroot, handle: Handle, run: SandboxRun) -> Self {
        Self {
            chroot,
            handle,
            run,
            console: Rc::new(RefCell::new(String::new())),
        }
    }

    fn run(&self, ctx: &AppContext) -> Result<SandboxOutput, AppError> {
        let modules = match &self.run.source {
            // TBD: This is dangerous, if accidentally used for anything else than the global JS tools it would allow breaking out of the chroot.
            SandboxSource::Function { directory, .. } => {
                module_loader::Modules::directory(ctx, directory)?
            }
            _ => module_loader::Modules::Chroot {
                chroot: self.chroot.clone(),
                handle: self.handle.clone(),
            },
        };
        let loader = Rc::new(module_loader::ChrootModuleLoader::new(ctx, modules));
        let mut context = self.get_context(ctx, loader.clone())?;

        let result = match &self.run.source {
            SandboxSource::Code(code) => helper::run_inline(&mut context, code),
            SandboxSource::File(path) => helper::run_default_export(&mut context, &loader, path),
            SandboxSource::Function {
                path,
                name,
                arguments,
                ..
            } => helper::run_function(&mut context, &loader, path, name, arguments),
        }
        .and_then(|value| helper::to_json(&mut context, &value));

        Ok(SandboxOutput {
            console: self.console.borrow().clone(),
            result,
        })
    }
}
