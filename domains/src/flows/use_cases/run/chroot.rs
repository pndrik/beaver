// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::sync::Arc;
use tracing::warn;

use crate::{
    app_error,
    core::{
        models::{
            AppContext, AppError, ErrorKind,
            file::{self, Chroot},
        },
        traits::FileSystem,
    },
};

type Fs = Arc<dyn FileSystem + Send + Sync>;

pub(super) async fn global(ctx: &AppContext, fs: &Fs) -> Result<Chroot, AppError> {
    Chroot::new(ctx, Arc::clone(fs), "/", false).await
}

pub(super) async fn prepare(
    ctx: &AppContext,
    fs: &Fs,
    path: &str,
    create: bool,
    delete: bool,
) -> Result<Chroot, AppError> {
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    let chroot = Chroot::new(ctx, Arc::clone(fs), &path, create).await?;

    if delete && chroot.root().is_root() {
        return Err(app_error!(
            Validation,
            "access_violation",
            &format!("Chroot '{path}' is the global chroot and can't be deleted"),
            ctx.clone()
        ));
    }

    Ok(chroot)
}

pub(super) async fn cleanup(ctx: &AppContext, fs: &Fs, chroot: &file::Path) {
    if chroot.is_root() {
        return;
    }

    let deleted = async {
        global(ctx, fs)
            .await?
            .unlink(ctx, chroot.as_str(), true)
            .await
    };
    match deleted.await {
        Ok(()) => {}
        Err(err) if err.kind == ErrorKind::NotFound => {}
        Err(err) => {
            warn!(chroot = %chroot, error = %err.internal_message, "Failed to delete chroot");
        }
    }
}
