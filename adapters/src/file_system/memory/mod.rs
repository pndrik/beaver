// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use regex::Regex;
use std::{
    collections::{BTreeMap, VecDeque},
    ops::{Range, RangeInclusive},
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
    time::{SystemTime, UNIX_EPOCH},
};

use super::errors::{exists, invalid, not_found};
use app_domains::{
    app_error,
    core::{
        models::{AppContext, AppError, file},
        traits::{FileSystem, LocalDirectory},
    },
};

const MAX_SYMLINK_HOPS: usize = 40;

/// The current time in seconds since the Unix epoch.
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

#[derive(Debug, Clone)]
enum Node {
    File(Vec<u8>, u64),
    Directory(u64),
    Symlink(file::Path, u64),
}

impl Node {
    fn metadata(&self) -> file::Metadata {
        let (kind, size, modified) = match self {
            Self::File(data, modified) => (file::Kind::File, data.len(), *modified),
            Self::Directory(modified) => (file::Kind::Directory, 0, *modified),
            Self::Symlink(target, modified) => {
                (file::Kind::Symlink, target.as_str().len(), *modified)
            }
        };
        file::Metadata {
            kind,
            size: size as u64,
            modified,
        }
    }
}

type Nodes = BTreeMap<file::Path, Node>;

#[derive(Debug)]
pub struct Memory {
    nodes: RwLock<Nodes>,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            nodes: RwLock::new(BTreeMap::from([(
                file::Path::root(),
                Node::Directory(now()),
            )])),
        }
    }

    fn read_nodes(&self) -> RwLockReadGuard<'_, Nodes> {
        self.nodes
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn write_nodes(&self) -> RwLockWriteGuard<'_, Nodes> {
        self.nodes
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

fn resolve(
    ctx: &AppContext,
    nodes: &Nodes,
    path: &file::Path,
    follow: bool,
) -> Result<file::Path, AppError> {
    let mut resolved = file::Path::root();
    let mut pending: VecDeque<String> = path.components().map(str::to_string).collect();
    let mut hops = 0;

    while let Some(name) = pending.pop_front() {
        let next = resolved.child(ctx, &name)?;
        let last = pending.is_empty();
        match nodes.get(&next) {
            Some(Node::Symlink(target, _)) if follow || !last => {
                hops += 1;
                if hops > MAX_SYMLINK_HOPS {
                    return Err(invalid(
                        ctx,
                        path.as_str(),
                        "has too many levels of symlinks",
                    ));
                }
                let mut expanded: VecDeque<String> =
                    target.components().map(str::to_string).collect();
                expanded.extend(pending);
                pending = expanded;
                resolved = file::Path::root();
            }
            Some(Node::Directory(_)) => resolved = next,
            _ if last => resolved = next,
            Some(_) => return Err(invalid(ctx, path.as_str(), "is not a directory")),
            None => return Err(not_found(ctx, path.as_str())),
        }
    }

    Ok(resolved)
}

fn existing<'a>(
    ctx: &AppContext,
    nodes: &'a Nodes,
    path: &file::Path,
    follow: bool,
) -> Result<(file::Path, &'a Node), AppError> {
    let resolved = resolve(ctx, nodes, path, follow)?;
    match nodes.get(&resolved) {
        Some(node) => Ok((resolved, node)),
        None => Err(not_found(ctx, path.as_str())),
    }
}

fn require_parent_directory(
    ctx: &AppContext,
    nodes: &Nodes,
    path: &file::Path,
) -> Result<(), AppError> {
    match path.parent().and_then(|parent| nodes.get(&parent)) {
        Some(Node::Directory(_)) => Ok(()),
        Some(_) => Err(invalid(ctx, path.as_str(), "has no directory as parent")),
        None => Err(not_found(ctx, path.as_str())),
    }
}

fn subtree<'a>(
    nodes: &'a Nodes,
    path: &'a file::Path,
) -> impl Iterator<Item = (&'a file::Path, &'a Node)> {
    nodes
        .range(path.clone()..)
        .take_while(move |(key, _)| key.starts_with(path))
}

fn subtree_paths(nodes: &Nodes, path: &file::Path) -> Vec<file::Path> {
    subtree(nodes, path).map(|(key, _)| key.clone()).collect()
}

fn mkdir_one(ctx: &AppContext, nodes: &mut Nodes, path: &file::Path) -> Result<(), AppError> {
    let resolved = resolve(ctx, nodes, path, false)?;
    if nodes.contains_key(&resolved) {
        return Err(exists(ctx, path.as_str()));
    }
    require_parent_directory(ctx, nodes, &resolved)?;
    nodes.insert(resolved, Node::Directory(now()));
    Ok(())
}

#[async_trait]
impl FileSystem for Memory {
    async fn metadata(
        &self,
        ctx: &AppContext,
        path: &file::Path,
    ) -> Result<file::Metadata, AppError> {
        let nodes = self.read_nodes();
        existing(ctx, &nodes, path, false).map(|(_, node)| node.metadata())
    }

    async fn canonicalize(
        &self,
        ctx: &AppContext,
        path: &file::Path,
    ) -> Result<file::Path, AppError> {
        let nodes = self.read_nodes();
        existing(ctx, &nodes, path, true).map(|(resolved, _)| resolved)
    }

    async fn read(
        &self,
        ctx: &AppContext,
        path: &file::Path,
        range: Option<Range<u64>>,
    ) -> Result<Vec<u8>, AppError> {
        let nodes = self.read_nodes();
        let Node::File(data, _) = existing(ctx, &nodes, path, true)?.1 else {
            return Err(invalid(ctx, path.as_str(), "is not a file"));
        };
        let Some(range) = range else {
            return Ok(data.clone());
        };
        let end = usize::try_from(range.end).map_or(data.len(), |end| end.min(data.len()));
        let start = usize::try_from(range.start).map_or(end, |start| start.min(end));
        Ok(data[start..end].to_vec())
    }

    async fn write(
        &self,
        ctx: &AppContext,
        path: &file::Path,
        data: &[u8],
        mode: file::WriteMode,
    ) -> Result<(), AppError> {
        let mut nodes = self.write_nodes();
        if mode == file::WriteMode::CreateNew {
            let link = resolve(ctx, &nodes, path, false)?;
            if nodes.contains_key(&link) {
                return Err(exists(ctx, path.as_str()));
            }
        }

        let resolved = resolve(ctx, &nodes, path, true)?;
        require_parent_directory(ctx, &nodes, &resolved)?;
        let mut content = match nodes.get(&resolved) {
            Some(Node::File(content, _)) => content.clone(),
            Some(_) => return Err(invalid(ctx, path.as_str(), "is not a file")),
            None => vec![],
        };
        match mode {
            file::WriteMode::Truncate | file::WriteMode::CreateNew => content = data.to_vec(),
            file::WriteMode::Append => content.extend_from_slice(data),
            file::WriteMode::At(offset) => {
                let (offset, end) = usize::try_from(offset)
                    .ok()
                    .and_then(|offset| Some((offset, offset.checked_add(data.len())?)))
                    .ok_or_else(|| {
                        invalid(ctx, path.as_str(), "has a write offset that is too large")
                    })?;
                if content.len() < end {
                    content.resize(end, 0);
                }
                content[offset..end].copy_from_slice(data);
            }
        }
        nodes.insert(resolved, Node::File(content, now()));
        Ok(())
    }

    async fn read_dir(
        &self,
        ctx: &AppContext,
        path: &file::Path,
    ) -> Result<Vec<file::Entry>, AppError> {
        let nodes = self.read_nodes();
        let (resolved, node) = existing(ctx, &nodes, path, true)?;
        if !matches!(node, Node::Directory(_)) {
            return Err(invalid(ctx, path.as_str(), "is not a directory"));
        }

        Ok(subtree(&nodes, &resolved)
            .filter(|(child, _)| child.parent().as_ref() == Some(&resolved))
            .map(|(child, node)| file::Entry {
                path: child.clone(),
                metadata: node.metadata(),
            })
            .collect())
    }

    async fn mkdir(
        &self,
        ctx: &AppContext,
        path: &file::Path,
        recursive: bool,
    ) -> Result<(), AppError> {
        let mut nodes = self.write_nodes();
        if !recursive {
            return mkdir_one(ctx, &mut nodes, path);
        }

        let mut current = file::Path::root();
        for name in path.components() {
            current = current.child(ctx, name)?;
            let resolved = resolve(ctx, &nodes, &current, true)?;
            match nodes.get(&resolved) {
                Some(Node::Directory(_)) => {}
                Some(_) => return Err(invalid(ctx, current.as_str(), "is not a directory")),
                None => mkdir_one(ctx, &mut nodes, &resolved)?,
            }
        }
        Ok(())
    }

    async fn copy(
        &self,
        ctx: &AppContext,
        from: &file::Path,
        to: &file::Path,
    ) -> Result<(), AppError> {
        let data = match existing(ctx, &self.read_nodes(), from, true)?.1 {
            Node::File(data, _) => data.clone(),
            _ => return Err(invalid(ctx, from.as_str(), "is not a file")),
        };
        self.write(ctx, to, &data, file::WriteMode::Truncate).await
    }

    async fn rename(
        &self,
        ctx: &AppContext,
        from: &file::Path,
        to: &file::Path,
    ) -> Result<(), AppError> {
        let mut nodes = self.write_nodes();
        let (source, node) = existing(ctx, &nodes, from, false)?;
        let is_directory = matches!(node, Node::Directory(_));
        let target = resolve(ctx, &nodes, to, false)?;
        require_parent_directory(ctx, &nodes, &target)?;
        if source == target {
            return Ok(());
        }
        if source.is_root() || target.starts_with(&source) {
            return Err(invalid(ctx, to.as_str(), "is inside of the source"));
        }
        // Like POSIX: a file replaces a file, a directory an empty directory.
        if let Some(existing) = nodes.get(&target)
            && (matches!(existing, Node::Directory(_)) != is_directory
                || subtree(&nodes, &target).nth(1).is_some())
        {
            return Err(invalid(ctx, to.as_str(), "can't be replaced"));
        }

        for path in subtree_paths(&nodes, &source) {
            let node = nodes.remove(&path).expect("listed from the map");
            let relative = path.strip_prefix(&source).expect("below the source");
            nodes.insert(target.join(&relative), node);
        }
        Ok(())
    }

    async fn symlink(
        &self,
        ctx: &AppContext,
        target: &file::Path,
        link: &file::Path,
    ) -> Result<(), AppError> {
        let mut nodes = self.write_nodes();
        let resolved = resolve(ctx, &nodes, link, false)?;
        if nodes.contains_key(&resolved) {
            return Err(exists(ctx, link.as_str()));
        }
        require_parent_directory(ctx, &nodes, &resolved)?;
        nodes.insert(resolved, Node::Symlink(target.clone(), now()));
        Ok(())
    }

    async fn read_link(&self, ctx: &AppContext, path: &file::Path) -> Result<file::Path, AppError> {
        match existing(ctx, &self.read_nodes(), path, false)?.1 {
            Node::Symlink(target, _) => Ok(target.clone()),
            _ => Err(invalid(ctx, path.as_str(), "is not a symlink")),
        }
    }

    async fn unlink(
        &self,
        ctx: &AppContext,
        path: &file::Path,
        recursive: bool,
    ) -> Result<(), AppError> {
        let mut nodes = self.write_nodes();
        let (resolved, _) = existing(ctx, &nodes, path, false)?;
        if resolved.is_root() {
            return Err(invalid(ctx, path.as_str(), "is the root"));
        }
        let removed = subtree_paths(&nodes, &resolved);
        if removed.len() > 1 && !recursive {
            return Err(invalid(ctx, path.as_str(), "is not empty"));
        }
        for path in removed {
            nodes.remove(&path);
        }
        Ok(())
    }

    /// Native: the subtree is one range of the map.
    async fn find(
        &self,
        ctx: &AppContext,
        path: &file::Path,
        pattern: Option<&Regex>,
        kinds: &[file::Kind],
        depth: RangeInclusive<usize>,
        exclude: &[String],
        limit: Option<usize>,
    ) -> Result<Vec<file::Entry>, AppError> {
        let nodes = self.read_nodes();
        let (start, _) = existing(ctx, &nodes, path, false)?;
        let start_level = start.components().count();

        let mut found = vec![];
        let mut skipped: Option<&file::Path> = None;
        for (key, node) in subtree(&nodes, &start) {
            if limit.is_some_and(|limit| found.len() >= limit) {
                break;
            }
            if skipped.is_some_and(|skipped| key.starts_with(skipped)) {
                continue;
            }
            let level = key.components().count() - start_level;
            let name = key.file_name().unwrap_or("/");
            let metadata = node.metadata();
            if level > 0 && exclude.iter().any(|excluded| excluded == name) {
                skipped = Some(key);
            } else if depth.contains(&level)
                && (kinds.is_empty() || kinds.contains(&metadata.kind))
                && pattern.is_none_or(|pattern| pattern.is_match(name))
            {
                found.push(file::Entry {
                    path: key.clone(),
                    metadata,
                });
            }
        }
        Ok(found)
    }

    async fn materialize(
        &self,
        ctx: &AppContext,
        path: &file::Path,
    ) -> Result<Box<dyn LocalDirectory + Send + Sync>, AppError> {
        Err(app_error!(
            Validation,
            "file_system_unsupported",
            &format!("The memory file system can't provide '{path}' as local directory"),
            ctx.clone()
        ))
    }
}
