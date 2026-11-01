// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use app_domains::{
    app_error,
    core::models::{AppContext, AppError, file, file::Chroot},
    inference::models::{Conversation, ConversationValue},
};

pub(super) struct State {
    path: file::Path,
    stored: Option<u64>,
    current: u64,
}

impl State {
    pub(super) async fn new(
        ctx: &AppContext,
        chroot: &Chroot,
        path: &file::Path,
        conversation: &Conversation,
    ) -> Result<Self, AppError> {
        Ok(Self {
            path: path.clone(),
            stored: stored(conversation, path).await,
            current: current(ctx, chroot, path).await?,
        })
    }

    pub(super) fn is_current(&self) -> bool {
        self.stored.unwrap_or(0) == self.current
    }

    pub(super) fn mark_read(&self, conversation: &mut Conversation) {
        conversation.set_value(
            key(&self.path),
            ConversationValue::Integer(i64::try_from(self.current).unwrap_or(0)),
        );
    }
}

fn key(path: &file::Path) -> String {
    format!("file_read:{path}")
}

async fn current(ctx: &AppContext, chroot: &Chroot, path: &file::Path) -> Result<u64, AppError> {
    let metadata = chroot.metadata(ctx, path.as_str()).await?;

    if metadata.is_dir() {
        return Err(app_error!(
            Validation,
            "file_is_directory",
            &format!("Path '{path}' is a directory"),
            ctx.clone()
        ));
    }

    Ok(metadata.modified)
}

async fn stored(conversation: &Conversation, path: &file::Path) -> Option<u64> {
    conversation
        .value(&key(path))
        .and_then(|value| match value {
            ConversationValue::Integer(stored) => Some(*stored),
            _ => None,
        })
        .and_then(|n| u64::try_from(n).ok())
}
