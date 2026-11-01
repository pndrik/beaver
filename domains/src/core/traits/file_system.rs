// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use async_trait::async_trait;
use regex::Regex;
use std::ops::{Range, RangeInclusive};

use super::LocalDirectory;
use crate::core::models::{AppContext, AppError, file};

#[async_trait]
pub trait FileSystem {
    async fn metadata(
        &self,
        ctx: &AppContext,
        path: &file::Path,
    ) -> Result<file::Metadata, AppError>;
    async fn canonicalize(
        &self,
        ctx: &AppContext,
        path: &file::Path,
    ) -> Result<file::Path, AppError>;

    async fn read(
        &self,
        ctx: &AppContext,
        path: &file::Path,
        range: Option<Range<u64>>,
    ) -> Result<Vec<u8>, AppError>;

    async fn write(
        &self,
        ctx: &AppContext,
        path: &file::Path,
        data: &[u8],
        mode: file::WriteMode,
    ) -> Result<(), AppError>;

    async fn read_dir(
        &self,
        ctx: &AppContext,
        path: &file::Path,
    ) -> Result<Vec<file::Entry>, AppError>;

    async fn mkdir(
        &self,
        ctx: &AppContext,
        path: &file::Path,
        recursive: bool,
    ) -> Result<(), AppError>;

    async fn copy(
        &self,
        ctx: &AppContext,
        from: &file::Path,
        to: &file::Path,
    ) -> Result<(), AppError>;

    async fn rename(
        &self,
        ctx: &AppContext,
        from: &file::Path,
        to: &file::Path,
    ) -> Result<(), AppError>;

    async fn symlink(
        &self,
        ctx: &AppContext,
        target: &file::Path,
        link: &file::Path,
    ) -> Result<(), AppError>;

    async fn read_link(&self, ctx: &AppContext, path: &file::Path) -> Result<file::Path, AppError>;

    async fn unlink(
        &self,
        ctx: &AppContext,
        path: &file::Path,
        recursive: bool,
    ) -> Result<(), AppError>;

    #[allow(clippy::too_many_arguments)]
    async fn find(
        &self,
        ctx: &AppContext,
        path: &file::Path,
        pattern: Option<&Regex>,
        kinds: &[file::Kind],
        depth: RangeInclusive<usize>,
        exclude: &[String],
        limit: Option<usize>,
    ) -> Result<Vec<file::Entry>, AppError>;

    async fn materialize(
        &self,
        ctx: &AppContext,
        path: &file::Path,
    ) -> Result<Box<dyn LocalDirectory + Send + Sync>, AppError>;
}
