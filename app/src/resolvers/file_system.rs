// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;

use app_adapters::file_system::{Disk, Memory};
use app_domains::{
    app_error,
    core::{
        models::{AppContext, AppError},
        traits::FileSystem,
    },
};

const CONFIGURATION_FILE_SYSTEM_PROVIDER: &str = "file_system.provider";
const CONFIGURATION_FILE_SYSTEM_DISK_ROOT: &str = "file_system.disk.root";

pub async fn file_system(ctx: &AppContext) -> Result<Arc<dyn FileSystem + Send + Sync>, AppError> {
    let configuration = ctx.configuration();
    let provider_name = configuration
        .get_string(ctx, CONFIGURATION_FILE_SYSTEM_PROVIDER)
        .await?;

    match provider_name.as_str() {
        "disk" => {
            let root = configuration
                .get_string(ctx, CONFIGURATION_FILE_SYSTEM_DISK_ROOT)
                .await?;
            Ok(Arc::new(Disk::new(ctx, &root)?))
        }
        "memory" => Ok(Arc::new(Memory::new())),
        _ => Err(app_error!(
            Internal,
            "bootstrap_failed",
            &format!("Unsupported file system provider: {provider_name}. Use 'disk' or 'memory'."),
            ctx.clone()
        )),
    }
}
