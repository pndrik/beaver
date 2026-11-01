// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use app_domains::{
    core::models::{AppContext, AppError, file},
    inference::models::Conversation,
    tools::models::ToolPermission,
};

/// `path` resolved in the chroot, if the scopes allow both.
pub(super) async fn resolve(
    ctx: &AppContext,
    conversation: &Conversation,
    permissions: &ToolPermission,
    path: &str,
) -> Result<file::Path, AppError> {
    permissions.scoped_path(ctx, path)?;
    let resolved = conversation.chroot().resolve(ctx, path).await?;
    permissions.scoped_path(ctx, resolved.as_str())
}

/// `path` with its parent resolved in the chroot and the last component as it
/// is, if the scopes allow both. For operations on a symlink itself.
pub(super) async fn leaf(
    ctx: &AppContext,
    conversation: &Conversation,
    permissions: &ToolPermission,
    path: &str,
) -> Result<file::Path, AppError> {
    let parsed = permissions.scoped_path(ctx, path)?;
    let (Some(parent), Some(name)) = (parsed.parent(), parsed.file_name()) else {
        return Ok(parsed);
    };
    let parent = conversation.chroot().resolve(ctx, parent.as_str()).await?;
    permissions.scoped_path(ctx, parent.child(ctx, name)?.as_str())
}
