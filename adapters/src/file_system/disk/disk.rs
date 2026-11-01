// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use cap_std::{
    ambient_authority,
    fs::{Dir, OpenOptions},
};
use regex::Regex;
use std::{
    io::{self, Read, Seek, SeekFrom, Write},
    ops::{Range, RangeInclusive},
    path::PathBuf,
    sync::Arc,
};

use super::{
    DiskDirectory,
    extensions::{AsBeaverError, AsBeaverFileMetadata, AsBeaverFilePath, AsRelativePathBuf},
    helpers::{self, absolute_target, relative_target, run},
};
use crate::file_system::errors::invalid;
use app_domains::core::{
    models::{AppContext, AppError, file},
    traits::{FileSystem, LocalDirectory},
};

#[derive(Debug)]
pub struct Disk {
    dir: Arc<Dir>,
    root: PathBuf,
}

impl Disk {
    pub fn new(ctx: &AppContext, root: &str) -> Result<Self, AppError> {
        let failed = |e: io::Error| e.as_beaver_error(ctx, &file::Path::root());

        let canonical = std::fs::canonicalize(root).map_err(failed)?;
        let dir = Dir::open_ambient_dir(&canonical, ambient_authority()).map_err(failed)?;

        Ok(Self {
            dir: Arc::new(dir),
            root: canonical,
        })
    }
}

#[async_trait]
impl FileSystem for Disk {
    async fn metadata(
        &self,
        ctx: &AppContext,
        path: &file::Path,
    ) -> Result<file::Metadata, AppError> {
        let relative = path.as_relative_path_buf();
        run(ctx, &self.dir, path, move |dir| {
            dir.symlink_metadata(&relative)?.as_beaver_file_metadata()
        })
        .await
    }

    async fn canonicalize(
        &self,
        ctx: &AppContext,
        path: &file::Path,
    ) -> Result<file::Path, AppError> {
        let relative = path.as_relative_path_buf();
        let canonical = run(ctx, &self.dir, path, move |dir| dir.canonicalize(&relative)).await?;
        canonical.as_beaver_file_path(ctx)
    }

    async fn read(
        &self,
        ctx: &AppContext,
        path: &file::Path,
        range: Option<Range<u64>>,
    ) -> Result<Vec<u8>, AppError> {
        let relative = path.as_relative_path_buf();
        let range = range.unwrap_or(0..u64::MAX);
        run(ctx, &self.dir, path, move |dir| {
            let mut file = dir.open(&relative)?;
            file.seek(SeekFrom::Start(range.start))?;
            let mut data = vec![];
            file.take(range.end.saturating_sub(range.start))
                .read_to_end(&mut data)?;
            Ok(data)
        })
        .await
    }

    async fn write(
        &self,
        ctx: &AppContext,
        path: &file::Path,
        data: &[u8],
        mode: file::WriteMode,
    ) -> Result<(), AppError> {
        let relative = path.as_relative_path_buf();
        let data = data.to_vec();
        run(ctx, &self.dir, path, move |dir| {
            let mut options = OpenOptions::new();
            options.write(true);
            match mode {
                file::WriteMode::Truncate => options.create(true).truncate(true),
                file::WriteMode::Append => options.create(true).append(true),
                file::WriteMode::At(_) => options.create(true),
                file::WriteMode::CreateNew => options.create_new(true),
            };
            let mut file = dir.open_with(&relative, &options)?;
            if let file::WriteMode::At(offset) = mode {
                file.seek(SeekFrom::Start(offset))?;
            }
            file.write_all(&data)
        })
        .await
    }

    async fn read_dir(
        &self,
        ctx: &AppContext,
        path: &file::Path,
    ) -> Result<Vec<file::Entry>, AppError> {
        let relative = path.as_relative_path_buf();
        let mut entries = run(ctx, &self.dir, path, move |dir| {
            dir.read_dir(&relative)?
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let name = entry.file_name().into_string().ok()?;
                    Some(
                        entry
                            .metadata()
                            .and_then(|metadata| metadata.as_beaver_file_metadata())
                            .map(|metadata| (name, metadata)),
                    )
                })
                .collect::<io::Result<Vec<_>>>()
        })
        .await?;
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        entries
            .into_iter()
            .map(|(name, metadata)| {
                Ok(file::Entry {
                    path: path.child(ctx, &name)?,
                    metadata,
                })
            })
            .collect()
    }

    async fn mkdir(
        &self,
        ctx: &AppContext,
        path: &file::Path,
        recursive: bool,
    ) -> Result<(), AppError> {
        let relative = path.as_relative_path_buf();
        run(ctx, &self.dir, path, move |dir| {
            if recursive {
                dir.create_dir_all(&relative)
            } else {
                dir.create_dir(&relative)
            }
        })
        .await
    }

    /// Not `Dir::copy`: on macOS it hands the target path to `fclonefileat`,
    /// which follows symlinks out of the root.
    async fn copy(
        &self,
        ctx: &AppContext,
        from: &file::Path,
        to: &file::Path,
    ) -> Result<(), AppError> {
        let (source, target) = (from.as_relative_path_buf(), to.as_relative_path_buf());
        run(ctx, &self.dir, from, move |dir| {
            let mut reader = dir.open(&source)?;
            if !reader.metadata()?.is_file() {
                return Err(io::Error::from(io::ErrorKind::IsADirectory));
            }
            let mut options = OpenOptions::new();
            options.write(true).create(true).truncate(true);
            io::copy(&mut reader, &mut dir.open_with(&target, &options)?).map(|_| ())
        })
        .await
    }

    async fn rename(
        &self,
        ctx: &AppContext,
        from: &file::Path,
        to: &file::Path,
    ) -> Result<(), AppError> {
        let (source, target) = (from.as_relative_path_buf(), to.as_relative_path_buf());
        run(ctx, &self.dir, from, move |dir| {
            dir.rename(&source, dir, &target)
        })
        .await
    }

    async fn symlink(
        &self,
        ctx: &AppContext,
        target: &file::Path,
        link: &file::Path,
    ) -> Result<(), AppError> {
        let content = relative_target(target, link);
        let relative = link.as_relative_path_buf();
        run(ctx, &self.dir, link, move |dir| {
            dir.symlink(&content, &relative)
        })
        .await
    }

    async fn read_link(&self, ctx: &AppContext, path: &file::Path) -> Result<file::Path, AppError> {
        let relative = path.as_relative_path_buf();
        let content = run(ctx, &self.dir, path, move |dir| {
            dir.read_link_contents(&relative)
        })
        .await?;
        absolute_target(ctx, path, &content)
    }

    async fn unlink(
        &self,
        ctx: &AppContext,
        path: &file::Path,
        recursive: bool,
    ) -> Result<(), AppError> {
        if path.is_root() {
            return Err(invalid(ctx, path.as_str(), "is the root"));
        }
        let relative = path.as_relative_path_buf();
        run(ctx, &self.dir, path, move |dir| {
            match dir.symlink_metadata(&relative)?.is_dir() {
                true if recursive => dir.remove_dir_all(&relative),
                true => dir.remove_dir(&relative),
                false => dir.remove_file(&relative),
            }
        })
        .await
    }

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
        helpers::find::find_by_walking(self, ctx, path, pattern, kinds, depth, exclude, limit).await
    }

    async fn materialize(
        &self,
        ctx: &AppContext,
        path: &file::Path,
    ) -> Result<Box<dyn LocalDirectory + Send + Sync>, AppError> {
        let canonical = self.canonicalize(ctx, path).await?;
        if !self.metadata(ctx, &canonical).await?.is_dir() {
            return Err(invalid(ctx, path.as_str(), "is not a directory"));
        }

        let host = self.root.join(canonical.as_relative_path_buf());
        let root = self.root.clone();
        let host = run(ctx, &self.dir, path, move |_| {
            let host = std::fs::canonicalize(host)?;
            if host.starts_with(&root) {
                Ok(host)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "materialized directory is outside of the root",
                ))
            }
        })
        .await?;

        Ok(Box::new(DiskDirectory { path: host }))
    }
}
