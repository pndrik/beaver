// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use boa_engine::{
    Context, JsObject, JsString, Module, Source, builtins::promise::PromiseState,
    module::SimpleModuleLoader,
};
use boa_runtime::{Console, console::DefaultLogger};
use std::{path::Path, rc::Rc};

use super::{FilteredFetcher, JavaScript, base64::register_base64};
use app_domains::{
    app_error,
    core::models::{AppContext, AppError},
};

impl JavaScript {
    pub(super) fn get_module_loader(
        &self,
        ctx: &AppContext,
    ) -> Result<Rc<SimpleModuleLoader>, AppError> {
        Ok(Rc::new(
            SimpleModuleLoader::new(Path::new(&self.directory)).map_err(|e| {
                app_error!(
                    Internal,
                    "skill_failed",
                    &format!("Failed to create module loader: {}", e),
                    ctx.clone()
                )
            })?,
        ))
    }

    fn get_fetcher(&self, _ctx: &AppContext) -> Result<FilteredFetcher, AppError> {
        let allowed_urls = self.scopes.fetch.clone().unwrap_or_default().urls;
        let allowed_methods = self.scopes.fetch.clone().unwrap_or_default().methods;

        Ok(FilteredFetcher::new(allowed_urls, allowed_methods))
    }

    pub(super) fn get_context(&mut self, ctx: &AppContext) -> Result<Context, AppError> {
        let loader = self.get_module_loader(ctx)?;
        let mut context = Context::builder()
            .module_loader(loader)
            .build()
            .map_err(|e| {
                app_error!(
                    Internal,
                    "skill_failed",
                    &format!("Failed to create JavaScript context: {}", e),
                    ctx.clone()
                )
            })?;

        Console::register_with_logger(DefaultLogger, &mut context).map_err(|e| {
            app_error!(
                Internal,
                "skill_failed",
                &format!("Failed to register console: {}", e),
                ctx.clone()
            )
        })?;

        register_base64(&mut context).map_err(|e| {
            app_error!(
                Internal,
                "skill_failed",
                &format!("Failed to register base64: {}", e),
                ctx.clone()
            )
        })?;

        if self.scopes.fetch.is_some() {
            boa_runtime::register(
                (boa_runtime::extensions::FetchExtension(
                    self.get_fetcher(ctx)?,
                ),),
                None,
                &mut context,
            )
            .map_err(|e| {
                app_error!(
                    Internal,
                    "skill_failed",
                    &format!("Failed to register fetch: {}", e),
                    ctx.clone()
                )
            })?;
        }

        Ok(context)
    }

    pub(super) fn get_module(
        &mut self,
        ctx: &AppContext,
        context: &mut Context,
    ) -> Result<Module, AppError> {
        let path = Path::new(&self.entrypoint);
        let source = Source::from_filepath(path).map_err(|e| {
            app_error!(
                Internal,
                "skill_failed",
                &format!("Failed to read entry module: {}", e),
                ctx.clone()
            )
        })?;

        let module = Module::parse(source, None, context).map_err(|e| {
            app_error!(
                Internal,
                "skill_failed",
                &format!("Failed to parse entry module: {}", e),
                ctx.clone()
            )
        })?;
        let promise = module.load_link_evaluate(context);
        context.run_jobs().map_err(|e| {
            app_error!(
                Internal,
                "skill_failed",
                &format!("Failed to run jobs: {}", e),
                ctx.clone()
            )
        })?;

        match promise.state() {
            PromiseState::Fulfilled(_) => {}
            PromiseState::Rejected(e) => {
                return Err(app_error!(
                    Internal,
                    "skill_failed",
                    &format!("Failed to load module: {}", e.display()),
                    ctx.clone()
                ));
            }
            PromiseState::Pending => {
                return Err(app_error!(
                    Internal,
                    "skill_failed",
                    "Module loading is still pending",
                    ctx.clone()
                ));
            }
        }

        Ok(module)
    }

    pub(super) fn get_function(
        &mut self,
        ctx: &AppContext,
        context: &mut Context,
        module: &Module,
        function: &str,
    ) -> Result<JsObject, AppError> {
        let namespace = module.namespace(context);
        let fn_object = namespace
            .get(JsString::from(function), context)
            .map_err(|e| {
                app_error!(
                    Internal,
                    "skill_failed",
                    &format!("Failed to get object '{}': {}", function, e),
                    ctx.clone()
                )
            })?;

        let fn_function = fn_object.as_callable().ok_or_else(|| {
            app_error!(
                Internal,
                "skill_failed",
                &format!("Object '{}' is not callable", function),
                ctx.clone()
            )
        })?;

        Ok(fn_function)
    }
}
