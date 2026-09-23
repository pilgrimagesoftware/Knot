//! An [`McpRunner`] that answers from a script instead of a subprocess.
//!
//! Every outcome this crate distinguishes is a shape of subprocess result, so
//! the tests script the result rather than installing an agent CLI.

use std::cell::RefCell;

use crate::error::{ProbeError, Result};
use crate::runner::{McpRunner, ProbeCommand};

type Answer = Box<dyn Fn(&ProbeCommand) -> Result<String>>;

/// Answers every call from one scripted result, recording what it was asked.
pub(crate) struct StubRunner {
    answer: Answer,
    calls:  RefCell<Vec<ProbeCommand>>,
}

impl StubRunner {
    pub(crate) fn new(answer: impl Fn(&ProbeCommand) -> Result<String> + 'static) -> Self {
        Self { answer: Box::new(answer),
               calls:  RefCell::new(Vec::new()), }
    }

    pub(crate) fn ok(stdout: &str) -> Self {
        let stdout = stdout.to_owned();
        Self::new(move |_| Ok(stdout.clone()))
    }

    pub(crate) fn missing(program: &str) -> Self {
        let program = program.to_owned();
        Self::new(move |_| Err(ProbeError::Missing { program: program.clone(), }))
    }

    pub(crate) fn failing(output: &str, code: i32) -> Self {
        let output = output.to_owned();
        Self::new(move |command| {
            Err(ProbeError::Command { program: command.label(),
                                      output: output.clone(),
                                      code })
        })
    }

    /// Every call made so far, in order.
    pub(crate) fn calls(&self) -> Vec<ProbeCommand> {
        self.calls.borrow().clone()
    }
}

impl McpRunner for StubRunner {
    fn run(&self, command: &ProbeCommand) -> Result<String> {
        self.calls.borrow_mut().push(command.clone());
        (self.answer)(command)
    }
}
