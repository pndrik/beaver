// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{fmt::Display, process};
use tracing::error;

pub fn fatal(message: &str, error: &dyn Display) -> ! {
    if tracing::dispatcher::has_been_set() {
        error!(error = %error, "{}", message);
    } else {
        eprintln!("{message}: {error}");
    }
    process::exit(1)
}
