// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::time::{SystemTime, UNIX_EPOCH};

use super::ConfigurationUniversal;
use app_domains::app_error;
use app_domains::core::models::{AppContext, AppError};

const CACHE_TTL_KEY: &str = "configuration.cache.ttl";

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[derive(Debug, Clone)]
pub(super) struct CacheObject {
    pub value_string: Option<String>,
    pub value_int: Option<i64>,
    pub value_bool: Option<bool>,
    pub created_at: u64,
}

impl ConfigurationUniversal {
    fn get_from_cache(&self, ctx: &AppContext, key: &str) -> Result<Option<CacheObject>, AppError> {
        Ok(self
            .cache
            .read()
            .map_err(|_| {
                app_error!(
                    Internal,
                    "configuration_load_failed",
                    "Failed to acquire cache lock",
                    ctx.clone()
                )
            })?
            .get(key)
            .cloned())
    }

    fn write_to_cache(
        &self,
        ctx: &AppContext,
        key: &str,
        value: CacheObject,
    ) -> Result<(), AppError> {
        self.cache
            .write()
            .map_err(|_| {
                app_error!(
                    Internal,
                    "configuration_load_failed",
                    "Failed to acquire cache lock",
                    ctx.clone()
                )
            })?
            .insert(key.to_string(), value);
        Ok(())
    }

    fn get_cache_ttl(&self, ctx: &AppContext) -> Result<u64, AppError> {
        if let Some(cached) = self.get_from_cache(ctx, CACHE_TTL_KEY)? {
            let ttl = cached.value_int.unwrap_or(0) as u64;
            if cached.created_at >= current_timestamp() - ttl {
                return Ok(ttl);
            }
        }

        let ttl = self.get_direct_int(ctx, CACHE_TTL_KEY)? as u64;
        self.write_to_cache(
            ctx,
            CACHE_TTL_KEY,
            CacheObject {
                value_string: None,
                value_int: Some(ttl as i64),
                value_bool: None,
                created_at: current_timestamp(),
            },
        )?;
        Ok(ttl)
    }

    pub(super) fn get_cached<T, FCache, Fetch>(
        &self,
        ctx: &AppContext,
        key: &str,
        from_cache: FCache,
        fetch: Fetch,
        into_cache: impl Fn(T) -> CacheObject,
    ) -> Result<T, AppError>
    where
        T: Clone,
        FCache: Fn(&CacheObject) -> Option<T>,
        Fetch: Fn(&Self, &AppContext, &str) -> Result<T, AppError>,
    {
        let ttl = self.get_cache_ttl(ctx)?;

        if let Some(value) = self
            .get_from_cache(ctx, key)?
            .filter(|entry| entry.created_at >= current_timestamp() - ttl)
            .and_then(|entry| from_cache(&entry))
        {
            return Ok(value);
        }

        let value = fetch(self, ctx, key)?;
        self.write_to_cache(ctx, key, into_cache(value.clone()))?;
        Ok(value)
    }
}
