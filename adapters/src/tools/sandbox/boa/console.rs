// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

use boa_engine::{Context, Finalize, JsResult, Trace};
use boa_runtime::{ConsoleState, Logger};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug, Trace, Finalize)]
pub(super) struct CapturedConsole {
    #[unsafe_ignore_trace]
    output: Rc<RefCell<String>>,
}

impl CapturedConsole {
    pub fn new(output: Rc<RefCell<String>>) -> Self {
        Self { output }
    }

    fn append(&self, msg: String, state: &ConsoleState) -> JsResult<()> {
        let mut output = self.output.borrow_mut();
        output.push_str(&" ".repeat(state.indent()));
        output.push_str(&msg);
        output.push('\n');
        Ok(())
    }
}

impl Logger for CapturedConsole {
    fn log(&self, msg: String, state: &ConsoleState, _context: &mut Context) -> JsResult<()> {
        self.append(msg, state)
    }

    fn info(&self, msg: String, state: &ConsoleState, _context: &mut Context) -> JsResult<()> {
        self.append(msg, state)
    }

    fn warn(&self, msg: String, state: &ConsoleState, _context: &mut Context) -> JsResult<()> {
        self.append(msg, state)
    }

    fn error(&self, msg: String, state: &ConsoleState, _context: &mut Context) -> JsResult<()> {
        self.append(msg, state)
    }
}
