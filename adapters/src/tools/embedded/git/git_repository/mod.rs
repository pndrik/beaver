// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use git2::{
    BranchType, Error, IndexAddOption,
    build::{CheckoutBuilder, RepoBuilder},
};
use std::path::PathBuf;

use super::Repository;

mod helpers;

pub struct GitRepository {
    pub path: PathBuf,
    pub repository: Repository,
}

impl GitRepository {
    pub fn new(repository: Repository, path: &str) -> Self {
        Self {
            path: PathBuf::from(path),
            repository,
        }
    }

    pub fn clone(&self) -> Result<(), Error> {
        RepoBuilder::new()
            .fetch_options(self.fetch_options())
            .clone(&self.repository.url, &self.path)?;

        Ok(())
    }

    pub fn pull(&self) -> Result<(), Error> {
        let repo = self.open()?;
        let mut remote = self.remote(&repo)?;

        let mut fo = self.fetch_options();
        remote.fetch(
            &["+refs/heads/*:refs/remotes/origin/*"],
            Some(&mut fo),
            None,
        )?;

        let head = repo.head()?;
        let branch = head.shorthand()?;

        let remote_oid = repo.refname_to_id(&Self::remote_ref(branch))?;
        let annotated = repo.find_annotated_commit(remote_oid)?;

        let (analysis, _) = repo.merge_analysis(&[&annotated])?;

        if analysis.is_up_to_date() {
            return Ok(());
        }

        if analysis.is_fast_forward() {
            let local_refname = Self::local_ref(branch);
            let mut reference = repo.find_reference(&local_refname)?;

            reference.set_target(annotated.id(), "pull: fast-forward")?;
            repo.set_head(&local_refname)?;
            repo.checkout_head(Some(CheckoutBuilder::default().force()))?;

            return Ok(());
        }

        Err(Error::from_str(
            "non-fast-forward: a real merge is required",
        ))
    }

    pub fn checkout(&self, branch: &str) -> Result<(), Error> {
        let repo = self.open()?;
        self.ensure_local_branch(&repo, branch)?;

        let local_refname = Self::local_ref(branch);
        let obj = repo.revparse_single(&local_refname)?;

        let mut opts = CheckoutBuilder::new();
        opts.force();
        repo.checkout_tree(&obj, Some(&mut opts))?;

        repo.set_head(&local_refname)?;

        Ok(())
    }

    pub fn commit(&self, message: &str) -> Result<(), Error> {
        let repo = self.open()?;

        let mut index = repo.index()?;
        index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
        index.write()?;

        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        let signature = self.signature()?;
        let parent_commit = repo
            .head()
            .ok()
            .and_then(|h| h.target())
            .and_then(|oid| repo.find_commit(oid).ok());

        let parent = parent_commit
            .as_ref()
            .map(|commit| commit as &git2::Commit)
            .into_iter()
            .collect::<Vec<_>>();

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parent,
        )?;

        Ok(())
    }

    pub fn push(&self) -> Result<(), Error> {
        let repo = self.open()?;
        let mut remote = self.remote(&repo)?;

        let head = repo.head()?;
        let branch = head.shorthand()?;
        let refname = Self::local_ref(branch);

        let mut push_options = self.push_options();
        remote.push(&[&format!("{refname}:{refname}")], Some(&mut push_options))?;

        repo.find_branch(branch, BranchType::Local)?
            .set_upstream(Some(branch))?;

        Ok(())
    }

    pub fn diff(&self, from: Option<&str>, to: Option<&str>) -> Result<String, Error> {
        let repo = self.open()?;

        let diff = match (from, to) {
            (None, None) => repo.diff_index_to_workdir(None, None)?,
            (Some(from), Some(to)) => {
                let old_tree = Self::resolve_tree(&repo, from)?;
                let new_tree = Self::resolve_tree(&repo, to)?;
                repo.diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)?
            }
            _ => {
                return Err(Error::from_str(
                    "both 'from' and 'to' must be provided together, or neither",
                ));
            }
        };

        Self::diff_to_string(&diff)
    }

    pub fn is_current_branch(&self, branch: &str) -> Result<bool, Error> {
        let repo = self.open()?;
        let head = repo.head()?;

        Ok(head.shorthand()? == branch)
    }

    pub fn delete_branch(&self, branch: &str) -> Result<(), Error> {
        let repo = self.open()?;

        repo.find_branch(branch, BranchType::Local)?.delete()
    }

    pub fn delete_remote_branch(&self, branch: &str) -> Result<(), Error> {
        let repo = self.open()?;
        let mut remote = self.remote(&repo)?;
        let mut push_options = self.push_options();

        remote.push(&[&format!(":refs/heads/{branch}")], Some(&mut push_options))
    }

    pub fn list_branches(&self, include_remote: bool) -> Result<Vec<(String, bool)>, Error> {
        let repo = self.open()?;
        let branch_type = if include_remote {
            None
        } else {
            Some(BranchType::Local)
        };

        let mut branches = Vec::new();
        for branch in repo.branches(branch_type)? {
            let (branch, _) = branch?;
            let name = branch.name()?.unwrap_or_default().to_string();
            branches.push((name, branch.is_head()));
        }

        Ok(branches)
    }
}
