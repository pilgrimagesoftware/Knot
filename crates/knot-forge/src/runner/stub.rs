//! A [`ForgeRunner`] that answers from a script instead of a subprocess.
//!
//! Every outcome this crate distinguishes is a shape of subprocess result, so
//! the tests script the result rather than the environment.

use std::cell::RefCell;

use crate::error::{ForgeError, Result};
use crate::runner::ForgeRunner;

type Answer = Box<dyn Fn(&[&str]) -> Result<String>>;

/// Answers every call from one scripted result, recording what it was asked.
pub(crate) struct StubRunner {
    answer: Answer,
    calls:  RefCell<Vec<String>>,
}

impl StubRunner {
    pub(crate) fn new(answer: impl Fn(&[&str]) -> Result<String> + 'static) -> Self {
        Self { answer: Box::new(answer),
               calls:  RefCell::new(Vec::new()), }
    }

    pub(crate) fn ok(stdout: &str) -> Self {
        let stdout = stdout.to_owned();
        Self::new(move |_| Ok(stdout.clone()))
    }

    pub(crate) fn missing() -> Self {
        Self::new(|_| Err(ForgeError::Missing))
    }

    pub(crate) fn failing(output: &str, code: i32) -> Self {
        let output = output.to_owned();
        Self::new(move |args| {
            Err(ForgeError::Command { command: args.join(" "),
                                      output: output.clone(),
                                      code })
        })
    }

    /// The arguments of every call made so far, joined, in order.
    pub(crate) fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }
}

impl ForgeRunner for StubRunner {
    fn run(&self, args: &[&str]) -> Result<String> {
        self.calls.borrow_mut().push(args.join(" "));
        (self.answer)(args)
    }
}
