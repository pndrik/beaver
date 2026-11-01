// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use regex::Regex;
use std::{
    fmt,
    ops::{Range, RangeInclusive},
    sync::Arc,
};

use super::{Entry, Kind, Metadata, Path, WriteMode};
use crate::{
    app_error,
    core::models::{AppContext, AppError, ErrorKind},
    core::traits::{FileSystem, LocalDirectory},
};

fn violation(ctx: &AppContext, message: &str) -> AppError {
    app_error!(Validation, "access_violation", message, ctx.clone())
}

/// One error for missing and outside, so errors don't tell whether something
/// outside of the chroot exists.
fn unreachable(ctx: &AppContext, path: &str) -> AppError {
    app_error!(
        NotFound,
        "file_not_found",
        &format!("Path '{path}' does not exist or is not accessible"),
        ctx.clone()
    )
}

#[derive(Clone)]
pub struct Chroot {
    fs: Arc<dyn FileSystem + Send + Sync>,
    root: Path,
}

impl fmt::Debug for Chroot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Chroot")
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

impl Chroot {
    pub async fn new(
        ctx: &AppContext,
        fs: Arc<dyn FileSystem + Send + Sync>,
        path: &str,
        create: bool,
    ) -> Result<Self, AppError> {
        let whole = Self {
            fs,
            root: Path::root(),
        };
        if create {
            whole.mkdir(ctx, path, true).await?;
        }
        let root = whole.existing_path(ctx, path).await?;
        if !whole.is_dir(ctx, &root).await? {
            return Err(violation(
                ctx,
                &format!("Chroot '{path}' is not a directory"),
            ));
        }
        Ok(Self { root, ..whole })
    }

    /// The root in the adapter's paths, e.g. for logs.
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub async fn metadata(&self, ctx: &AppContext, path: &str) -> Result<Metadata, AppError> {
        let absolute = if Path::parse(ctx, path)?.is_root() {
            self.existing_path(ctx, path).await?
        } else {
            self.link_path(ctx, path).await?
        };
        self.fs
            .metadata(ctx, &absolute)
            .await
            .map_err(|e| self.relabel(e))
    }

    pub async fn read(
        &self,
        ctx: &AppContext,
        path: &str,
        range: Option<Range<u64>>,
    ) -> Result<Vec<u8>, AppError> {
        let absolute = self.existing_path(ctx, path).await?;
        self.fs
            .read(ctx, &absolute, range)
            .await
            .map_err(|e| self.relabel(e))
    }

    pub async fn read_to_string(&self, ctx: &AppContext, path: &str) -> Result<String, AppError> {
        String::from_utf8(self.read(ctx, path, None).await?).map_err(|_| {
            app_error!(
                Validation,
                "file_not_text",
                &format!("File '{path}' is not valid UTF-8"),
                ctx.clone()
            )
        })
    }

    pub async fn write(
        &self,
        ctx: &AppContext,
        path: &str,
        data: &[u8],
        mode: WriteMode,
    ) -> Result<(), AppError> {
        let absolute = self.write_path(ctx, path).await?;
        self.fs
            .write(ctx, &absolute, data, mode)
            .await
            .map_err(|e| self.relabel(e))
    }

    pub async fn resolve(&self, ctx: &AppContext, path: &str) -> Result<Path, AppError> {
        let absolute = match self.existing_path(ctx, path).await {
            Err(err) if err.kind == ErrorKind::NotFound => self.link_path(ctx, path).await?,
            absolute => absolute?,
        };
        Ok(self.relative(&absolute))
    }

    pub async fn read_dir(&self, ctx: &AppContext, path: &str) -> Result<Vec<Entry>, AppError> {
        let absolute = self.existing_path(ctx, path).await?;
        let entries = self.fs.read_dir(ctx, &absolute).await;
        Ok(self.relative_entries(entries.map_err(|e| self.relabel(e))?))
    }

    pub async fn mkdir(
        &self,
        ctx: &AppContext,
        path: &str,
        recursive: bool,
    ) -> Result<(), AppError> {
        if !recursive {
            let absolute = self.link_path(ctx, path).await?;
            return self
                .fs
                .mkdir(ctx, &absolute, false)
                .await
                .map_err(|e| self.relabel(e));
        }

        let mut current = self.existing_path(ctx, "/").await?;
        for name in Path::parse(ctx, path)?.components() {
            let next = current.child(ctx, name)?;
            if self.existing(ctx, &next).await?.is_none() {
                self.fs
                    .mkdir(ctx, &next, false)
                    .await
                    .map_err(|e| self.relabel(e))?;
            }
            current = self.contain(ctx, path, &next).await?;
            if !self.is_dir(ctx, &current).await? {
                return Err(app_error!(
                    Conflict,
                    "file_exists",
                    &format!("A parent of '{path}' is not a directory"),
                    ctx.clone()
                ));
            }
        }
        Ok(())
    }

    pub async fn copy(&self, ctx: &AppContext, from: &str, to: &str) -> Result<(), AppError> {
        let from = self.existing_path(ctx, from).await?;
        let to = self.write_path(ctx, to).await?;
        self.fs
            .copy(ctx, &from, &to)
            .await
            .map_err(|e| self.relabel(e))
    }

    pub async fn rename(&self, ctx: &AppContext, from: &str, to: &str) -> Result<(), AppError> {
        let from = self.link_path(ctx, from).await?;
        let to = self.link_path(ctx, to).await?;
        self.fs
            .rename(ctx, &from, &to)
            .await
            .map_err(|e| self.relabel(e))
    }

    pub async fn symlink(
        &self,
        ctx: &AppContext,
        target: &str,
        link: &str,
    ) -> Result<(), AppError> {
        let absolute_target = if Path::parse(ctx, target)?.is_root() {
            self.existing_path(ctx, target).await?
        } else {
            self.link_path(ctx, target).await?
        };
        if self.existing(ctx, &absolute_target).await?.is_some() {
            self.contain(ctx, target, &absolute_target).await?;
        }
        let absolute_link = self.link_path(ctx, link).await?;
        self.fs
            .symlink(ctx, &absolute_target, &absolute_link)
            .await
            .map_err(|e| self.relabel(e))
    }

    pub async fn read_link(&self, ctx: &AppContext, path: &str) -> Result<Path, AppError> {
        let absolute = self.link_path(ctx, path).await?;
        let target = self.fs.read_link(ctx, &absolute).await;
        target
            .map_err(|e| self.relabel(e))?
            .strip_prefix(&self.root)
            .ok_or_else(|| violation(ctx, &format!("Symlink '{path}' points outside of chroot")))
    }

    pub async fn unlink(
        &self,
        ctx: &AppContext,
        path: &str,
        recursive: bool,
    ) -> Result<(), AppError> {
        if Path::parse(ctx, path)?.is_root() {
            return Err(violation(
                ctx,
                &format!("Path '{path}' is the chroot and can't be deleted"),
            ));
        }
        let absolute = self.link_path(ctx, path).await?;
        self.fs
            .unlink(ctx, &absolute, recursive)
            .await
            .map_err(|e| self.relabel(e))
    }

    pub async fn materialize(
        &self,
        ctx: &AppContext,
        path: &str,
    ) -> Result<Box<dyn LocalDirectory + Send + Sync>, AppError> {
        let absolute = self.existing_path(ctx, path).await?;
        self.fs
            .materialize(ctx, &absolute)
            .await
            .map_err(|e| self.relabel(e))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn find(
        &self,
        ctx: &AppContext,
        path: &str,
        pattern: Option<&Regex>,
        kinds: &[Kind],
        depth: RangeInclusive<usize>,
        exclude: &[String],
        limit: Option<usize>,
    ) -> Result<Vec<Entry>, AppError> {
        let start = self.existing_path(ctx, path).await?;
        let found = self
            .fs
            .find(ctx, &start, pattern, kinds, depth, exclude, limit)
            .await;
        Ok(self.relative_entries(found.map_err(|e| self.relabel(e))?))
    }

    async fn existing_path(&self, ctx: &AppContext, path: &str) -> Result<Path, AppError> {
        let absolute = self.root.join(&Path::parse(ctx, path)?);
        self.contain(ctx, path, &absolute).await
    }

    async fn link_path(&self, ctx: &AppContext, path: &str) -> Result<Path, AppError> {
        let relative = Path::parse(ctx, path)?;
        let (Some(parent), Some(name)) = (relative.parent(), relative.file_name()) else {
            return Err(violation(ctx, &format!("Path '{path}' has no file name")));
        };
        self.existing_path(ctx, parent.as_str())
            .await?
            .child(ctx, name)
    }

    async fn write_path(&self, ctx: &AppContext, path: &str) -> Result<Path, AppError> {
        let target = self.link_path(ctx, path).await?;
        if !self
            .is_dir(ctx, &target.parent().unwrap_or_else(Path::root))
            .await?
        {
            return Err(violation(
                ctx,
                &format!("Parent of '{path}' is not a directory"),
            ));
        }
        if self.existing(ctx, &target).await?.is_none() {
            return Ok(target);
        }

        let canonical = self.contain(ctx, path, &target).await?;
        if self.is_dir(ctx, &canonical).await? {
            Err(violation(ctx, &format!("Path '{path}' is a directory")))
        } else {
            Ok(canonical)
        }
    }

    async fn contain(
        &self,
        ctx: &AppContext,
        path: &str,
        absolute: &Path,
    ) -> Result<Path, AppError> {
        match self.fs.canonicalize(ctx, absolute).await {
            Ok(canonical) if canonical.starts_with(&self.root) => Ok(canonical),
            _ => Err(unreachable(ctx, path)),
        }
    }

    async fn existing(
        &self,
        ctx: &AppContext,
        absolute: &Path,
    ) -> Result<Option<Metadata>, AppError> {
        match self.fs.metadata(ctx, absolute).await {
            Err(err) if err.kind == ErrorKind::NotFound => Ok(None),
            metadata => metadata.map(Some).map_err(|e| self.relabel(e)),
        }
    }

    async fn is_dir(&self, ctx: &AppContext, absolute: &Path) -> Result<bool, AppError> {
        let metadata = self.existing(ctx, absolute).await?;
        Ok(metadata.is_some_and(|metadata| metadata.is_dir()))
    }

    fn relative(&self, absolute: &Path) -> Path {
        absolute.strip_prefix(&self.root).unwrap_or_else(Path::root)
    }

    fn relative_entries(&self, entries: Vec<Entry>) -> Vec<Entry> {
        entries
            .into_iter()
            .map(|entry| Entry {
                path: self.relative(&entry.path),
                ..entry
            })
            .collect()
    }

    fn relabel(&self, mut err: AppError) -> AppError {
        if !self.root.is_root() {
            let root = self.root.as_str();
            err.internal_message = err
                .internal_message
                .replace(&format!("'{root}'"), "'/'")
                .replace(&format!("'{root}/"), "'/");
        }
        err
    }
}
