// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use config::{Config, ConfigBuilder, Environment, File, FileFormat, builder::DefaultState};
use std::{
    collections::HashMap,
    sync::RwLock,
    time::{SystemTime, UNIX_EPOCH},
};

use app_domains::{
    app_error,
    core::{
        models::{AppContext, AppError},
        traits::Configuration,
    },
};

mod cache;
use cache::CacheObject;
mod direct;

const DEFAULTS: &str = include_str!("../defaults.yaml");

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[derive(Debug)]
pub struct ConfigurationUniversal {
    cache: RwLock<HashMap<String, CacheObject>>,
    builder: ConfigBuilder<DefaultState>,
}

fn get_home_directory() -> String {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|| "".to_string())
}

fn get_configuration_file() -> String {
    std::env::var_os("CONFIGURATION__PATH")
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("{}/.config/beaver/configuration.yaml", get_home_directory()))
}

impl ConfigurationUniversal {
    pub fn new() -> Result<Self, AppError> {
        let builder = Config::builder()
            .add_source(File::from_str(DEFAULTS, FileFormat::Yaml))
            .add_source(File::new(&get_configuration_file(), FileFormat::Yaml).required(false))
            .add_source(File::new(".env", FileFormat::Ini).required(false))
            .add_source(Environment::default().separator("__").ignore_empty(true));

        Ok(Self {
            builder,
            cache: RwLock::new(HashMap::new()),
        })
    }

    fn get_config(&self, ctx: &AppContext) -> Result<Config, AppError> {
        self.builder.clone().build().map_err(|e| {
            app_error!(
                Internal,
                "configuration_load_failed",
                &format!("Failed to build configuration: {}", e),
                ctx.clone()
            )
        })
    }
}

#[async_trait]
impl Configuration for ConfigurationUniversal {
    async fn get_string(&self, ctx: &AppContext, key: &str) -> Result<String, AppError> {
        self.get_cached(
            ctx,
            key,
            |entry| entry.value_string.clone(),
            Self::get_direct_string,
            |value| CacheObject {
                value_string: Some(value),
                value_int: None,
                value_bool: None,
                created_at: current_timestamp(),
            },
        )
    }

    async fn get_int(&self, ctx: &AppContext, key: &str) -> Result<i64, AppError> {
        self.get_cached(
            ctx,
            key,
            |entry| entry.value_int,
            Self::get_direct_int,
            |value| CacheObject {
                value_string: None,
                value_int: Some(value),
                value_bool: None,
                created_at: current_timestamp(),
            },
        )
    }

    async fn get_bool(&self, ctx: &AppContext, key: &str) -> Result<bool, AppError> {
        self.get_cached(
            ctx,
            key,
            |entry| entry.value_bool,
            Self::get_direct_bool,
            |value| CacheObject {
                value_string: None,
                value_int: None,
                value_bool: Some(value),
                created_at: current_timestamp(),
            },
        )
    }

    async fn get_map(
        &self,
        ctx: &AppContext,
        key: &str,
    ) -> Result<HashMap<String, String>, AppError> {
        let config = self.get_config(ctx)?;
        let map = config.get_table(key).map_err(|e| {
            app_error!(
                Internal,
                "configuration_load_failed",
                &format!("Failed to get map for key '{}': {}", key, e),
                ctx.clone()
            )
        })?;

        let result = map
            .into_iter()
            .map(|(k, v)| (k, v.into_string().unwrap_or_default()))
            .collect::<HashMap<String, String>>();

        Ok(result)
    }
}
