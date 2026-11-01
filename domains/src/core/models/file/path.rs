// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use std::{cmp::Ordering, fmt};

use crate::app_error;
use crate::core::models::{AppContext, AppError};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path(String);

fn invalid(ctx: &AppContext, message: &str) -> AppError {
    app_error!(Validation, "access_violation", message, ctx.clone())
}

impl Path {
    pub fn root() -> Self {
        Self("/".to_string())
    }

    pub fn parse(ctx: &AppContext, value: &str) -> Result<Self, AppError> {
        if value.contains('\0') {
            return Err(invalid(ctx, "Path must not contain NUL"));
        }
        if !value.starts_with('/') {
            return Err(invalid(ctx, &format!("Path must be absolute: '{value}'")));
        }

        let mut normalized = String::new();
        for component in value.split('/') {
            match component {
                "" | "." => {}
                ".." => {
                    return Err(invalid(
                        ctx,
                        &format!("Path must not contain '..': '{value}'"),
                    ));
                }
                name => {
                    normalized.push('/');
                    normalized.push_str(name);
                }
            }
        }

        if normalized.is_empty() {
            Ok(Self::root())
        } else {
            Ok(Self(normalized))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_root(&self) -> bool {
        self.0 == "/"
    }

    pub fn components(&self) -> impl Iterator<Item = &str> {
        self.0.split('/').filter(|component| !component.is_empty())
    }

    #[must_use]
    pub fn join(&self, other: &Path) -> Path {
        match (self.is_root(), other.is_root()) {
            (true, _) => other.clone(),
            (_, true) => self.clone(),
            _ => Self(format!("{}{}", self.0, other.0)),
        }
    }

    pub fn child(&self, ctx: &AppContext, name: &str) -> Result<Path, AppError> {
        if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\0']) {
            return Err(invalid(ctx, &format!("Invalid file name: '{name}'")));
        }
        Ok(self.join(&Self(format!("/{name}"))))
    }

    pub fn parent(&self) -> Option<Path> {
        let (parent, _) = self.0.rsplit_once('/').filter(|_| !self.is_root())?;
        Some(match parent {
            "" => Self::root(),
            parent => Self(parent.to_string()),
        })
    }

    pub fn file_name(&self) -> Option<&str> {
        self.0
            .rsplit_once('/')
            .map(|(_, name)| name)
            .filter(|name| !name.is_empty())
    }

    pub fn starts_with(&self, prefix: &Path) -> bool {
        prefix.is_root()
            || self.0 == prefix.0
            || self
                .0
                .strip_prefix(&prefix.0)
                .is_some_and(|rest| rest.starts_with('/'))
    }

    pub fn strip_prefix(&self, root: &Path) -> Option<Path> {
        if root.is_root() {
            return Some(self.clone());
        }
        match self.0.strip_prefix(&root.0)? {
            "" => Some(Self::root()),
            rest if rest.starts_with('/') => Some(Self(rest.to_string())),
            _ => None,
        }
    }
}

impl Ord for Path {
    fn cmp(&self, other: &Self) -> Ordering {
        self.components().cmp(other.components())
    }
}

impl PartialOrd for Path {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
