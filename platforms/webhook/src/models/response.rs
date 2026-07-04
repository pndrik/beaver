// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use serde::Serialize;

#[derive(Serialize)]
pub struct Response {
    pub success: bool,
    pub message: String,
}
