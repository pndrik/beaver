// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use regex::Regex;
use std::ops::RangeInclusive;

use app_domains::core::{
    models::{AppContext, AppError, file},
    traits::FileSystem,
};

fn matches(
    entry: &file::Entry,
    level: usize,
    pattern: Option<&Regex>,
    kinds: &[file::Kind],
    depth: &RangeInclusive<usize>,
) -> bool {
    depth.contains(&level)
        && (kinds.is_empty() || kinds.contains(&entry.metadata.kind))
        && pattern.is_none_or(|pattern| pattern.is_match(entry.path.file_name().unwrap_or("/")))
}

fn excluded(entry: &file::Entry, exclude: &[String]) -> bool {
    entry
        .path
        .file_name()
        .is_some_and(|name| exclude.iter().any(|excluded| excluded == name))
}

#[allow(clippy::too_many_arguments)]
pub(in crate::file_system) async fn find_by_walking<F: FileSystem + Sync + ?Sized>(
    fs: &F,
    ctx: &AppContext,
    path: &file::Path,
    pattern: Option<&Regex>,
    kinds: &[file::Kind],
    depth: RangeInclusive<usize>,
    exclude: &[String],
    limit: Option<usize>,
) -> Result<Vec<file::Entry>, AppError> {
    let metadata = fs.metadata(ctx, path).await?;

    let mut found = vec![];
    let mut pending = vec![(
        file::Entry {
            path: path.clone(),
            metadata,
        },
        0,
    )];
    while let Some((entry, level)) = pending.pop() {
        if limit.is_some_and(|limit| found.len() >= limit) {
            break;
        }
        if entry.metadata.is_dir() && level < *depth.end() {
            match fs.read_dir(ctx, &entry.path).await {
                Ok(children) => pending.extend(
                    children
                        .into_iter()
                        .rev()
                        .filter(|child| !excluded(child, exclude))
                        .map(|child| (child, level + 1)),
                ),
                Err(err) if level == 0 => return Err(err),
                Err(_) => {}
            }
        }
        if matches(&entry, level, pattern, kinds, &depth) {
            found.push(entry);
        }
    }
    Ok(found)
}
