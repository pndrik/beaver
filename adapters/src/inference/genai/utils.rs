// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use genai::chat::CacheControl;

use app_domains::inference::models::MessageCacheLevel;

pub(super) fn convert_cache_control(cache_control: &MessageCacheLevel) -> Option<CacheControl> {
    match cache_control {
        MessageCacheLevel::Ephemeral5min => Some(CacheControl::Ephemeral5m),
        MessageCacheLevel::Ephemeral1h => Some(CacheControl::Ephemeral1h),
        MessageCacheLevel::Ephemeral24h => Some(CacheControl::Ephemeral24h),
        _ => None,
    }
}
