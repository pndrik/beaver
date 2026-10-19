// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

mod branch_delete;
pub(super) use branch_delete::BranchDelete;

mod branches;
pub(super) use branches::Branches;

mod checkout;
pub(super) use checkout::Checkout;

mod clone;
pub(super) use clone::Clone;

mod commit;
pub(super) use commit::Commit;

mod diff;
pub(super) use diff::Diff;

mod log;
pub(super) use log::Log;

mod pull;
pub(super) use pull::Pull;

mod push;
pub(super) use push::Push;

mod show;
pub(super) use show::Show;

/// `output` cut to `max` lines, with a note how many were left out.
fn truncate_lines(output: String, max: usize) -> String {
    let lines: Vec<&str> = output.lines().collect();
    if lines.len() <= max {
        return output;
    }

    format!(
        "{}\n... (truncated, {} more lines)",
        lines[..max].join("\n"),
        lines.len() - max
    )
}
