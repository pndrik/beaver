// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use cap_std::fs::Dir;
use std::{
    io,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use super::extensions::AsBeaverError;
use crate::file_system::errors::{escape, failed, invalid};
use app_domains::core::models::{AppContext, AppError, file};

pub(in crate::file_system) mod find;

pub(super) async fn run<T, F>(
    ctx: &AppContext,
    dir: &Arc<Dir>,
    path: &file::Path,
    operation: F,
) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce(&Dir) -> io::Result<T> + Send + 'static,
{
    let dir = Arc::clone(dir);
    tokio::task::spawn_blocking(move || operation(&dir))
        .await
        .map_err(|e| failed(ctx, "run the file system task for", path.as_str(), e))?
        .map_err(|e| e.as_beaver_error(ctx, path))
}

pub(super) fn relative_target(target: &file::Path, link: &file::Path) -> PathBuf {
    let parent = link.parent().unwrap_or_else(file::Path::root);
    let from: Vec<&str> = parent.components().collect();
    let to: Vec<&str> = target.components().collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();

    let relative: PathBuf = std::iter::repeat_n("..", from.len() - common)
        .chain(to[common..].iter().copied())
        .collect();
    if relative.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        relative
    }
}

pub(super) fn absolute_target(
    ctx: &AppContext,
    link: &file::Path,
    content: &Path,
) -> Result<file::Path, AppError> {
    let parent = link.parent().unwrap_or_else(file::Path::root);
    let mut components: Vec<&str> = parent.components().collect();
    for component in content.components() {
        match component {
            Component::Normal(name) => match name.to_str() {
                Some(name) => components.push(name),
                None => return Err(invalid(ctx, link.as_str(), "is not valid UTF-8")),
            },
            Component::CurDir => {}
            Component::ParentDir if components.pop().is_some() => {}
            _ => return Err(escape(ctx, link.as_str())),
        }
    }
    file::Path::parse(ctx, &format!("/{}", components.join("/")))
}
