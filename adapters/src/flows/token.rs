// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use subtle::ConstantTimeEq;

use app_domains::flows::models::Definition;

pub(super) fn has_webhook(flow: &Definition, name: &str) -> bool {
    flow.spec
        .triggers
        .webhook
        .as_ref()
        .is_some_and(|webhook| webhook.name == name)
}

/// Whether `token` matches the plain text token of the flow's webhook trigger.
/// A flow without a token accepts nothing.
///
/// The comparison uses `subtle` because `==` returns at the first differing
/// byte, so response times would let a caller recover the token byte by byte.
pub(super) fn token_matches(flow: &Definition, token: &str) -> bool {
    flow.spec.triggers.webhook.as_ref().is_some_and(|webhook| {
        !webhook.token.is_empty() && bool::from(webhook.token.as_bytes().ct_eq(token.as_bytes()))
    })
}
