// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteMode {
    Truncate,
    Append,
    At(u64),
    CreateNew,
}
