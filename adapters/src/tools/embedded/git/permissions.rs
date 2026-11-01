// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use app_domains::{
    app_error,
    core::{
        models::{AppContext, AppError, ErrorKind},
        traits::LocalDirectory,
    },
    inference::models::Conversation,
    tools::models::ToolPermission,
};

pub(super) async fn repository(
    ctx: &AppContext,
    conversation: &Conversation,
    permissions: &ToolPermission,
    path: &str,
) -> Result<Box<dyn LocalDirectory + Send + Sync>, AppError> {
    permissions.scoped_path(ctx, path)?;
    let resolved = conversation.chroot().resolve(ctx, path).await?;
    permissions.scoped_path(ctx, resolved.as_str())?;
    conversation
        .chroot()
        .materialize(ctx, resolved.as_str())
        .await
}

pub(super) async fn clone_target(
    ctx: &AppContext,
    conversation: &Conversation,
    permissions: &ToolPermission,
    path: &str,
) -> Result<(Box<dyn LocalDirectory + Send + Sync>, String), AppError> {
    let parsed = permissions.scoped_path(ctx, path)?;
    let (Some(parent), Some(name)) = (parsed.parent(), parsed.file_name()) else {
        return Err(app_error!(
            Validation,
            "tool_failed",
            &format!("Path '{path}' can't be cloned into"),
            ctx.clone()
        ));
    };

    let chroot = conversation.chroot();
    match chroot.metadata(ctx, path).await {
        Err(err) if err.kind == ErrorKind::NotFound => {}
        Err(err) => return Err(err),
        Ok(_) => {
            return Err(app_error!(
                Validation,
                "tool_failed",
                &format!("Path '{path}' already exists"),
                ctx.clone()
            ));
        }
    }

    chroot.mkdir(ctx, parent.as_str(), true).await?;
    let resolved = chroot.resolve(ctx, parent.as_str()).await?;
    permissions.scoped_path(ctx, resolved.as_str())?;
    let local = chroot.materialize(ctx, resolved.as_str()).await?;
    Ok((local, name.to_string()))
}
