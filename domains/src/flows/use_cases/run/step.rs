// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde_json::{Value, json};

use super::{FlowRun, chroot, values};
use crate::{
    app_error,
    core::models::{AppContext, AppError},
    flows::{
        Flows,
        models::{Action, Definition, Step},
    },
    inference::Inference,
    tools::Tools,
};

impl Flows {
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn step(
        &self,
        ctx: &AppContext,
        flow: &Definition,
        step: &Step,
        values: &Value,
        inference: &Inference,
        tools: &Tools,
        run: &mut FlowRun,
    ) -> Result<(), AppError> {
        let name = step.context.as_deref().unwrap_or_default();
        if !run.contexts.contains_key(name) {
            let declaration = flow.spec.contexts.get(name).ok_or_else(|| {
                app_error!(
                    Validation,
                    "flow_invalid",
                    &format!("Context '{}' is not declared", name),
                    ctx.clone()
                )
            })?;
            let context = self
                .context(
                    ctx,
                    declaration,
                    values,
                    inference,
                    tools,
                    &mut run.delete_on_finish,
                )
                .await?;
            run.contexts.insert(name.to_string(), context);
        }

        let context = run
            .contexts
            .get_mut(name)
            .expect("context was just created");

        let output = match &step.action {
            Action::Agent(arguments) => {
                self.agent(ctx, context, arguments, values, inference, tools)
                    .await?
            }
            Action::Tool(arguments) => self.tool(ctx, context, arguments, values, tools).await?,
            Action::Loop(_) => unreachable!("loops are run by `repeat`"),
        };

        let global = chroot::global_chroot(ctx).await?;
        let messages = values::messages(context.conversation.messages());
        run.steps.insert(
            step.name.clone(),
            json!({
                "output": output,
                "context": name,
                "chroot": chroot::relative(&global, &context.chroot),
                "messages": messages,
            }),
        );

        Ok(())
    }
}
