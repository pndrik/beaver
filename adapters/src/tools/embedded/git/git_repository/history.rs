// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use chrono::{DateTime, FixedOffset};
use git2::{Commit, Delta, Diff, DiffOptions, DiffStatsFormat, Error, Oid, Repository, Sort, Time};
use std::path::Path;

use super::GitRepository;

fn format_time(time: Time) -> String {
    FixedOffset::east_opt(time.offset_minutes() * 60)
        .and_then(|offset| {
            DateTime::from_timestamp(time.seconds(), 0).map(|date| date.with_timezone(&offset))
        })
        .map(|date| date.format("%Y-%m-%d %H:%M:%S %z").to_string())
        .unwrap_or_else(|| time.seconds().to_string())
}

fn entry_id(commit: &Commit, file: &str) -> Result<Option<Oid>, Error> {
    Ok(commit
        .tree()?
        .get_path(Path::new(file))
        .ok()
        .map(|entry| entry.id()))
}

fn touches(commit: &Commit, file: &str) -> Result<bool, Error> {
    let entry = entry_id(commit, file)?;
    match commit.parent_count() {
        0 => Ok(entry.is_some()),
        _ => Ok(entry_id(&commit.parent(0)?, file)? != entry),
    }
}

const MAX_LOG_FILES: usize = 20;

fn parent_diff<'r>(
    repo: &'r Repository,
    commit: &Commit,
    file: Option<&str>,
) -> Result<Diff<'r>, Error> {
    let parent_tree = match commit.parent_count() {
        0 => None,
        _ => Some(commit.parent(0)?.tree()?),
    };
    let mut options = DiffOptions::new();
    if let Some(file) = file {
        options.pathspec(file);
    }

    let mut diff = repo.diff_tree_to_tree(
        parent_tree.as_ref(),
        Some(&commit.tree()?),
        Some(&mut options),
    )?;
    diff.find_similar(None)?;
    Ok(diff)
}

fn changed_files(diff: &Diff) -> Vec<String> {
    let path = |file: git2::DiffFile| {
        file.path()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default()
    };

    let mut files: Vec<String> = diff
        .deltas()
        .take(MAX_LOG_FILES)
        .map(|delta| match delta.status() {
            Delta::Renamed => format!(
                "  R {} -> {}",
                path(delta.old_file()),
                path(delta.new_file())
            ),
            Delta::Added => format!("  A {}", path(delta.new_file())),
            Delta::Deleted => format!("  D {}", path(delta.old_file())),
            Delta::Copied => format!("  C {}", path(delta.new_file())),
            Delta::Typechange => format!("  T {}", path(delta.new_file())),
            _ => format!("  M {}", path(delta.new_file())),
        })
        .collect();
    if diff.deltas().len() > MAX_LOG_FILES {
        files.push(format!(
            "  ... {} more files",
            diff.deltas().len() - MAX_LOG_FILES
        ));
    }
    files
}

impl GitRepository {
    pub fn log(
        &self,
        revision: &str,
        file: Option<&str>,
        skip: usize,
        limit: usize,
    ) -> Result<Vec<String>, Error> {
        let repo = self.open()?;
        let start = repo.revparse_single(revision)?.peel_to_commit()?;

        let mut walk = repo.revwalk()?;
        walk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;
        walk.push(start.id())?;

        let mut commits = Vec::new();
        let mut skipped = 0;
        for id in walk {
            if commits.len() == limit {
                break;
            }

            let commit = repo.find_commit(id?)?;
            if let Some(file) = file
                && !touches(&commit, file)?
            {
                continue;
            }
            if skipped < skip {
                skipped += 1;
                continue;
            }

            let id = commit.id().to_string();
            let mut entry = vec![format!(
                "{} {} {} {}",
                &id[..10],
                format_time(commit.time()),
                commit.author().name().unwrap_or_default(),
                commit.summary().ok().flatten().unwrap_or_default()
            )];
            entry.extend(changed_files(&parent_diff(&repo, &commit, None)?));
            commits.push(entry.join("\n"));
        }

        Ok(commits)
    }

    pub fn show(&self, revision: &str, file: Option<&str>) -> Result<String, Error> {
        let repo = self.open()?;
        let commit = repo.revparse_single(revision)?.peel_to_commit()?;

        let diff = parent_diff(&repo, &commit, file)?;

        let author = commit.author();
        let mut output = format!("commit {}\n", commit.id());
        if commit.parent_count() > 1 {
            let parents: Vec<String> = commit.parent_ids().map(|id| id.to_string()).collect();
            output.push_str(&format!("Merge: {}\n", parents.join(" ")));
        }
        output.push_str(&format!(
            "Author: {} <{}>\nDate:   {}\n\n",
            author.name().unwrap_or_default(),
            author.email().unwrap_or_default(),
            format_time(author.when())
        ));
        for line in commit.message().unwrap_or_default().trim_end().lines() {
            output.push_str(&format!("    {}\n", line));
        }

        let stats = diff.stats()?.to_buf(DiffStatsFormat::FULL, 80)?;
        output.push('\n');
        output.push_str(stats.as_str().unwrap_or_default());
        output.push('\n');
        output.push_str(&Self::diff_to_string(&diff)?);

        Ok(output)
    }
}
