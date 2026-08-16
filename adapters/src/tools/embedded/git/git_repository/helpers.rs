// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use git2::{
    BranchType, CertificateCheckStatus, Cred, Diff, DiffFormat, Error, FetchOptions, PushOptions,
    Remote, RemoteCallbacks, Repository as Git2Repository, Signature, Tree,
};
use ssh2::{CheckResult, KnownHostFileKind, Session};

use super::GitRepository;

impl GitRepository {
    pub(super) fn local_ref(branch: &str) -> String {
        format!("refs/heads/{branch}")
    }

    pub(super) fn remote_ref(branch: &str) -> String {
        format!("refs/remotes/origin/{branch}")
    }

    pub(super) fn signature(&self) -> Result<Signature<'static>, Error> {
        Signature::now(
            self.repository
                .display_name
                .as_deref()
                .unwrap_or(&self.repository.email),
            &self.repository.email,
        )
    }

    pub(super) fn remote_callbacks(&self) -> RemoteCallbacks<'static> {
        let username = self.repository.username.clone();
        let password = self.repository.password.clone();
        let ssh_key = self.repository.ssh_key.clone();

        let mut callbacks = RemoteCallbacks::new();
        let mut offered_password = false;
        let mut offered_ssh_key = false;

        callbacks.credentials(move |_url, username_from_url, allowed| {
            let username = username.as_deref().or(username_from_url).unwrap_or("git");

            if !offered_password
                && allowed.is_user_pass_plaintext()
                && let Some(pw) = password.as_deref()
            {
                offered_password = true;
                return Cred::userpass_plaintext(username, pw);
            }
            if !offered_ssh_key
                && allowed.is_ssh_key()
                && let Some(key) = ssh_key.as_deref()
            {
                offered_ssh_key = true;
                return Cred::ssh_key_from_memory(username, None, key, None);
            }
            if allowed.is_username() {
                return Cred::username(username);
            }
            Err(Error::from_str("no usable credentials"))
        });

        let known_hosts = self.repository.known_hosts.clone();
        callbacks.certificate_check(move |cert, host| {
            if let Some(Some(hostkey)) = cert.as_hostkey().map(|hostkey| hostkey.hostkey()) {
                let mut ssh2_known_hosts = Session::new().unwrap().known_hosts().unwrap();
                for known_host in &known_hosts {
                    ssh2_known_hosts
                        .read_str(&known_host, KnownHostFileKind::OpenSSH)
                        .unwrap();
                }

                match ssh2_known_hosts.check(host, hostkey) {
                    CheckResult::Match => {
                        return Ok(CertificateCheckStatus::CertificateOk);
                    }
                    CheckResult::Mismatch => {
                        return Err(Error::from_str(&format!(
                            "ssh host key mismatch for host '{}'",
                            host
                        )));
                    }
                    CheckResult::NotFound => {
                        return Err(Error::from_str(&format!(
                            "ssh host key not found for host '{}'",
                            host
                        )));
                    }
                    CheckResult::Failure => {
                        return Err(Error::from_str(&format!(
                            "failed to check ssh host key for host '{}'",
                            host
                        )));
                    }
                }
            }

            if cert.as_x509().is_some() {
                return Ok(CertificateCheckStatus::CertificatePassthrough);
            }

            Err(Error::from_str("invalid host certificate"))
        });

        callbacks
    }

    pub(super) fn fetch_options(&self) -> FetchOptions<'static> {
        let mut fo = FetchOptions::new();
        fo.remote_callbacks(self.remote_callbacks());

        fo
    }

    pub(super) fn push_options(&self) -> PushOptions<'static> {
        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(self.remote_callbacks());

        push_options
    }

    pub(super) fn open(&self) -> Result<Git2Repository, Error> {
        Git2Repository::open(&self.path)
    }

    pub(super) fn remote<'a>(&self, repository: &'a Git2Repository) -> Result<Remote<'a>, Error> {
        repository.remote_anonymous(&self.repository.url)
    }

    pub(super) fn resolve_tree<'r>(repo: &'r Git2Repository, rev: &str) -> Result<Tree<'r>, Error> {
        repo.revparse_single(rev)?.peel_to_tree()
    }

    pub(super) fn diff_to_string(diff: &Diff) -> Result<String, Error> {
        let mut output = String::new();

        diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
            match line.origin() {
                '+' | '-' | ' ' => output.push(line.origin()),
                _ => {}
            }
            output.push_str(&String::from_utf8_lossy(line.content()));
            true
        })?;

        Ok(output)
    }

    pub(super) fn ensure_local_branch(
        &self,
        repo: &Git2Repository,
        branch: &str,
    ) -> Result<(), Error> {
        if repo.refname_to_id(&Self::local_ref(branch)).is_ok() {
            return Ok(());
        }

        if let Ok(remote_oid) = repo.refname_to_id(&Self::remote_ref(branch)) {
            let commit = repo.find_commit(remote_oid)?;
            repo.branch(branch, &commit, false)?;
            repo.find_branch(branch, BranchType::Local)?
                .set_upstream(Some(branch))?;
        } else {
            let head_commit = repo.head().and_then(|h| h.peel_to_commit())?;
            repo.branch(branch, &head_commit, false)?;
        }

        Ok(())
    }
}
