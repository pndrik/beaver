// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use git2::{ConfigLevel, Error, Repository, RepositoryOpenFlags};
use regex::Regex;
use std::{
    ffi::OsStr,
    path::Path,
    sync::{LazyLock, Once},
};

#[cfg(test)]
mod tests;

static ISOLATE: Once = Once::new();

/// `[user@]host:path`, the host at least two characters so `C:` isn't one.
static SCP_URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:[A-Za-z0-9._-]+@)?[A-Za-z0-9][A-Za-z0-9.-]+:[^:]").expect("valid regex")
});

/// Keeps libgit2 from reading system, global and XDG configuration. Every
/// git tool calls it before it touches libgit2.
pub(super) fn isolate() {
    ISOLATE.call_once(|| {
        for level in [
            ConfigLevel::System,
            ConfigLevel::Global,
            ConfigLevel::XDG,
            ConfigLevel::ProgramData,
        ] {
            // SAFETY: Setting search paths races with other libgit2 calls. It
            // runs once, and every libgit2 use of this crate waits for it.
            unsafe {
                let _ = git2::opts::set_search_path(level, "");
            }
        }
    });
}

fn outside(what: &str) -> Error {
    Error::from_str(&format!("{what} is outside of the repository directory"))
}

/// Opens the repository at `directory` itself, never a parent, and refuses it
/// if its git directory, worktree, objects or configuration lead outside of
/// `directory`. Checkouts write symlinks as regular files.
pub(super) fn open_contained(directory: &Path) -> Result<Repository, Error> {
    isolate();
    let directory = directory
        .canonicalize()
        .map_err(|e| Error::from_str(&e.to_string()))?;
    let repository = Repository::open_ext(
        &directory,
        RepositoryOpenFlags::NO_SEARCH,
        [] as [&OsStr; 0],
    )?;

    let inside = |path: &Path| {
        path.canonicalize()
            .is_ok_and(|path| path.starts_with(&directory))
    };
    if !inside(repository.path()) {
        return Err(outside("The git directory"));
    }
    if repository.workdir().is_some_and(|workdir| !inside(workdir)) {
        return Err(outside("The worktree"));
    }
    for file in ["objects/info/alternates", "commondir"] {
        if repository.path().join(file).symlink_metadata().is_ok() {
            return Err(Error::from_str(&format!(
                "Repositories with '{file}' are not supported"
            )));
        }
    }

    let mut config = repository.config()?.open_level(ConfigLevel::Local)?;
    let mut refused = None;
    config.entries(None)?.for_each(|entry| {
        let name = entry.name().unwrap_or_default().to_lowercase();
        if name == "core.worktree" || name.starts_with("include.") || name.starts_with("includeif.")
        {
            refused.get_or_insert(name);
        }
    })?;
    if let Some(name) = refused {
        return Err(Error::from_str(&format!(
            "Repositories with '{name}' are not supported"
        )));
    }
    config.set_bool("core.symlinks", false)?;

    Ok(repository)
}

/// Only `https://`, `ssh://` and scp-like `[user@]host:path` URLs, no local
/// paths, `file://` or other transports.
pub(super) fn validate_url(url: &str) -> Result<(), String> {
    let remote = url.starts_with("https://")
        || url.starts_with("ssh://")
        || (!url.contains("://") && SCP_URL.is_match(url));
    if remote {
        Ok(())
    } else {
        Err(format!(
            "Repository URL '{url}' is not allowed, use https://, ssh:// or user@host:path"
        ))
    }
}
