// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Map, Value, json};
use std::{collections::HashMap, path::PathBuf, sync::Arc};

use super::{Action, Definition, RunContext, Step};
use crate::{
    app_error,
    core::{
        models::{AppContext, AppError},
        traits::TemplateEngine,
    },
};

const FLOW_CALL_MAX_DEPTH: u32 = 8;

pub(crate) struct FlowRun {
    pub(crate) flow: Arc<Definition>,
    pub(crate) contexts: HashMap<String, RunContext>,
    pub(crate) arguments: Map<String, Value>,
    pub(crate) variables: Map<String, Value>,
    pub(crate) flows: Map<String, Value>,
    pub(crate) delete_on_finish: Vec<PathBuf>,
    pub(crate) depth: u32,
}

impl FlowRun {
    pub(crate) fn new(flow: Definition, arguments: Map<String, Value>, depth: u32) -> Self {
        Self {
            flow: Arc::new(flow),
            contexts: HashMap::new(),
            arguments,
            variables: Map::new(),
            flows: Map::new(),
            delete_on_finish: Vec::new(),
            depth,
        }
    }

    pub(crate) fn child(
        &self,
        ctx: &AppContext,
        flow: Definition,
        raw: &Map<String, Value>,
    ) -> Result<FlowRun, AppError> {
        if self.depth >= FLOW_CALL_MAX_DEPTH {
            return Err(app_error!(
                Validation,
                "flow_call_too_deep",
                &format!(
                    "Calling flow '{}' exceeds the maximum depth of {}",
                    flow.metadata.name, FLOW_CALL_MAX_DEPTH
                ),
                ctx.clone()
            ));
        }
        let arguments = flow.arguments(ctx, raw)?;
        Ok(FlowRun::new(flow, arguments, self.depth + 1))
    }

    pub(crate) fn called(&mut self, step: &Step, child: FlowRun) {
        self.flows
            .insert(step.name.clone(), Value::Object(child.variables));
    }

    pub(crate) fn delete_when_finished(&mut self, chroot: PathBuf) {
        if !self.delete_on_finish.contains(&chroot) {
            self.delete_on_finish.push(chroot);
        }
    }

    pub(crate) fn template_values(&self, ctx: &AppContext) -> Value {
        let contexts: Map<String, Value> = self
            .contexts
            .iter()
            .map(|(name, context)| (name.clone(), context.messages()))
            .collect();
        json!({
            "Arguments": self.arguments,
            "Variables": self.variables,
            "Contexts": contexts,
            "Flows": self.flows,
            "Ctx": ctx,
            "Flow": { "name": self.flow.metadata.name },
        })
    }

    pub(crate) fn context_mut(&mut self, step: &Step) -> &mut RunContext {
        self.contexts
            .get_mut(step.context_name())
            .expect("contexts are created when the run starts")
    }

    pub(crate) fn step_message(&self, step: &Step, message: &str) -> String {
        format!(
            "Flow '{}' step '{}' {}",
            self.flow.metadata.name, step.name, message
        )
    }

    pub(crate) fn failed(&self, step: &Step, mut err: AppError) -> AppError {
        err.internal_message =
            self.step_message(step, &format!("failed: {}", err.internal_message));
        err
    }

    pub(crate) async fn set(
        &mut self,
        ctx: &AppContext,
        engine: &(dyn TemplateEngine + Send + Sync),
        step: &Step,
    ) -> Result<(), AppError> {
        let Action::Set(arguments) = &step.action else {
            unreachable!("set runs set steps")
        };
        let values = self.template_values(ctx);
        for (name, variable) in &arguments.0 {
            let rendered = engine.render(ctx, &variable.value, &values).await?;
            let value = match variable.resolve(ctx, &rendered) {
                Ok(value) => value,
                Err(err) if err.translation_key == "flow_variable_invalid" => {
                    let fallback = variable.fallback();
                    ctx.logger
                        .warn(
                            ctx,
                            &format!(
                                "Flow '{}' step '{}': variable '{}': {}, using default {}",
                                self.flow.metadata.name,
                                step.name,
                                name,
                                err.internal_message,
                                fallback
                            ),
                        )
                        .await;
                    fallback
                }
                Err(err) => return Err(err),
            };
            ctx.logger
                .trace(ctx, &format!("[{} = {}]", name, value))
                .await;
            self.variables.insert(name.clone(), value);
        }
        Ok(())
    }
}
