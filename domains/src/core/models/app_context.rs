// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::path::Path;
use std::sync::Arc;

use super::AppError;
use crate::app_error;
use crate::core::traits::Configuration;

#[derive(Debug, Clone)]
pub struct AppContext {
    pub trace_id: String,
    pub chroot: String,
    pub configuration: Arc<dyn Configuration + Send + Sync>,
}

impl AppContext {
    pub fn new(
        trace_id: String,
        chroot: String,
        configuration: Arc<dyn Configuration + Send + Sync>,
    ) -> Result<Self, AppError> {
        let chroot = Path::new(chroot.trim_end_matches('/'))
            .canonicalize()
            .map_err(|e| {
                app_error!(
                    Validation,
                    "access_violation",
                    &format!("Invalid chroot '{}': {}", chroot, e),
                    AppContext {
                        trace_id: trace_id.clone(),
                        chroot,
                        configuration: configuration.clone(),
                    }
                )
            })?
            .to_string_lossy()
            .to_string();

        Ok(Self {
            trace_id,
            chroot,
            configuration,
        })
    }

    pub fn get_absolute_path(&self, path: &str) -> Result<String, AppError> {
        let joined = format!(
            "{}/{}",
            self.chroot.trim_end_matches('/'),
            path.trim_start_matches('/')
        );

        let canonicalized = Path::new(&joined).canonicalize().map_err(|e| {
            app_error!(
                Validation,
                "access_violation",
                &format!("Invalid path '{}': {}", path, e),
                self.clone()
            )
        })?;

        let chroot_canonicalized = Path::new(&self.chroot).canonicalize().map_err(|e| {
            app_error!(
                Validation,
                "access_violation",
                &format!("Invalid chroot '{}': {}", self.chroot, e),
                self.clone()
            )
        })?;
        if !canonicalized.starts_with(&chroot_canonicalized) {
            return Err(app_error!(
                Validation,
                "access_violation",
                &format!("Path '{}' is outside of chroot", path),
                self.clone()
            ));
        }

        if !canonicalized.exists() {
            return Err(app_error!(
                Validation,
                "access_violation",
                &format!("Path '{}' does not exist", path),
                self.clone()
            ));
        }

        Ok(canonicalized.to_string_lossy().to_string())
    }
}
