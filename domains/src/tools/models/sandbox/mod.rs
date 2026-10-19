// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

//! Requests to and results of a [`crate::tools::traits::CodeSandbox`].

mod fetch_scope;
pub use fetch_scope::*;

mod filesystem_operation;
pub use filesystem_operation::*;

mod filesystem_scope;
pub use filesystem_scope::*;

mod limits;
pub use limits::*;

mod output;
pub use output::*;

mod run;
pub use run::*;

mod scopes;
pub use scopes::*;

mod source;
pub use source::*;
