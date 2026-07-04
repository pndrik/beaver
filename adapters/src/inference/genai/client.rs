// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::time::Duration;

use genai::{
    Client, ModelIden, ServiceTarget, WebConfig,
    adapter::AdapterKind,
    resolver::{AuthData, Endpoint, ServiceTargetResolver},
};

use super::GenAi;
use app_domains::{
    core::models::{AppContext, AppError},
    inference::models::{Model, ModelAdapter},
};

impl GenAi {
    async fn get_endpoint(&self, ctx: &AppContext) -> Result<Endpoint, AppError> {
        let endpoint = ctx
            .configuration
            .get_string(ctx, &self.configuration_key_endpoint)
            .await?;
        Ok(Endpoint::from_owned(endpoint))
    }

    async fn get_apikey(&self, ctx: &AppContext) -> Result<String, AppError> {
        ctx.configuration
            .get_string(ctx, &self.configuration_key_api_key)
            .await
    }

    pub(super) async fn get_client(&self, ctx: &AppContext) -> Result<Client, AppError> {
        let endpoint = self.get_endpoint(ctx).await?;
        let auth = AuthData::Key(self.get_apikey(ctx).await?);

        let target_resolver = ServiceTargetResolver::from_resolver_fn(
            |service_target: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
                let ServiceTarget { model, .. } = service_target;

                let model_domain = Model::from_id(&model.model_name).ok_or_else(|| {
                    genai::resolver::Error::Custom(format!("Unknown model: {}", model.model_name))
                })?;
                let adapter_kind = match model_domain.adapter() {
                    ModelAdapter::OpenAI => AdapterKind::OpenAI,
                    ModelAdapter::OpenAIResp => AdapterKind::OpenAIResp,
                    ModelAdapter::Anthropic => AdapterKind::Anthropic,
                    ModelAdapter::Gemini => AdapterKind::Gemini,
                };

                let model = ModelIden::new(adapter_kind, model.model_name.clone());

                Ok(ServiceTarget {
                    endpoint,
                    auth,
                    model,
                })
            },
        );

        Ok(Client::builder()
            .with_service_target_resolver(target_resolver)
            .with_web_config(
                WebConfig::default()
                    .with_timeout(Duration::from_secs(90))
                    .with_connect_timeout(Duration::from_secs(5)),
            )
            .build())
    }
}
