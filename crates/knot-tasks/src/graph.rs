//! The committed plan: a directed acyclic graph of tasks, the readiness
//! rule that drives them, and the gate that stops one being dispatched
//! before what it depends on is finished.
//!
//! Contract: `openspec/specs/task-graph/spec.md`.
//!
//! The graph resolves nothing. It refers to agents by id and to
//! capabilities by tag, and hands both back to its caller untouched. That
//! is what lets the rules here be tested without a registry, a message
//! queue or a runtime - and what stops a re-plan quietly re-pointing a task
//! whose work is already under way.

use std::collections::{BTreeMap, BTreeSet};

use uuid::Uuid;

use crate::consts::MAX_TASKS;
use crate::error::{Result, TaskError};
use crate::task::{Assignee, Outcome, Task, TaskId, TaskSpec, TaskState};

/// What the gate says about a task a caller wants to dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dispatchable {
    pub id:       TaskId,
    pub goal:     String,
    /// `None` when the plan recorded the work without deciding who does it.
    /// The caller has to choose before it can deliver anything.
    pub assignee: Option<Assignee>,
}

/// What completing a task changed elsewhere in the plan.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Completion {
    /// Tasks that became dispatchable because this one finished.
    pub now_ready:   Vec<TaskId>,
    /// Tasks that will not run as planned because this one failed.
    pub now_blocked: Vec<TaskId>,
}

/// A committed plan.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskGraph {
    /// Submission order, preserved so a rendered plan reads the way it was
    /// written.
    tasks: Vec<Task>,
}

impl TaskGraph {
    /// Validates and commits a fresh plan.
    ///
    /// Validation happens here rather than at dispatch: a plan that cannot
    /// be run to completion is not worth dispatching the first task of.
    pub fn commit(specs: Vec<TaskSpec>) -> Result<Self> {
        validate(&specs)?;
        let mut graph = Self { tasks: specs.into_iter().map(Task::from_spec).collect(), };
        graph.refresh_readiness();
        Ok(graph)
    }

    /// Commits a revised plan over this one.
    ///
    /// A task present in both keeps the state it had, so revising a plan
    /// mid-flight neither cancels nor re-sends work already under way, and
    /// does not re-run what is already done. Tasks only in the old plan are
    /// dropped; tasks only in the new one start by the usual rule.
    pub fn recommit(&self, specs: Vec<TaskSpec>) -> Result<Self> {
        validate(&specs)?;
        let carried: BTreeMap<&TaskId, &Task> =
            self.tasks.iter().map(|task| (&task.id, task)).collect();
        let tasks = specs.into_iter()
                         .map(|spec| {
                             let previous = carried.get(&spec.id).copied();
                             let mut task = Task::from_spec(spec);
                             if let Some(previous) = previous {
                                 task.state = previous.state;
                                 task.dispatched_to = previous.dispatched_to;
                             }
                             task
                         })
                         .collect();
        let mut graph = Self { tasks };
        graph.refresh_readiness();
        Ok(graph)
    }

    pub fn tasks(&self) -> &[Task] {
        &self.tasks
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn task(&self, id: &TaskId) -> Option<&Task> {
        self.tasks.iter().find(|task| task.id == *id)
    }

    /// Whether this plan covers work that must go through a graph at all.
    ///
    /// Work is nontrivial when it spans more than one agent, or more than
    /// one task. A single hand-off to a single agent is an ordinary
    /// message; wrapping it in a plan would be ceremony, since there is no
    /// ordering to record.
    pub fn is_nontrivial(&self) -> bool {
        if self.tasks.len() > 1 {
            return true;
        }
        self.distinct_agent_assignees().len() > 1
    }

    fn distinct_agent_assignees(&self) -> BTreeSet<Uuid> {
        self.tasks
            .iter()
            .filter_map(|task| match &task.assignee {
                Some(Assignee::Agent(id)) => Some(*id),
                _ => None,
            })
            .collect()
    }

    /// Asks whether `id` may be dispatched now.
    ///
    /// Answers with what to deliver and to whom, or with the dependencies
    /// that are not yet done - naming them, so the caller learns what it is
    /// waiting for rather than only that it may not proceed. Reads only;
    /// the caller marks the task dispatched once delivery actually
    /// succeeds.
    pub fn gate(&self, id: &TaskId) -> Result<Dispatchable> {
        let task = self.task(id)
                       .ok_or_else(|| TaskError::NotFound(id.clone()))?;
        match task.state {
            TaskState::Ready => Ok(Dispatchable { id:       task.id.clone(),
                                                  goal:     task.goal.clone(),
                                                  assignee: task.assignee.clone(), }),
            TaskState::Pending => Err(TaskError::DependenciesUnmet { task:  id.clone(),
                                                                     unmet: self.unmet(task), }),
            other => Err(TaskError::NotReady { task:  id.clone(),
                                               state: other.as_str(), }),
        }
    }

    /// Records that `id` was delivered to `agent`.
    ///
    /// Called only after delivery succeeded. A delivery the messaging rules
    /// reject leaves the task `Ready`, so it can be dispatched again once
    /// the caller has somewhere to send it.
    pub fn mark_dispatched(&mut self, id: &TaskId, agent: Uuid) -> Result<()> {
        self.gate(id)?;
        let task = self.task_mut(id).expect("gate proved it is present");
        task.state = TaskState::Dispatched;
        task.dispatched_to = Some(agent);
        Ok(())
    }

    /// Reports how a dispatched task turned out, and applies what that
    /// changes downstream.
    pub fn complete(&mut self, id: &TaskId, outcome: Outcome) -> Result<Completion> {
        let task = self.task(id)
                       .ok_or_else(|| TaskError::NotFound(id.clone()))?;
        if task.state != TaskState::Dispatched {
            return Err(TaskError::NotDispatched { task:  id.clone(),
                                                  state: task.state.as_str(), });
        }
        let before = self.states();
        self.task_mut(id).expect("checked above").state = match outcome {
            Outcome::Done => TaskState::Done,
            Outcome::Failed => TaskState::Failed,
        };
        if outcome == Outcome::Failed {
            self.block_dependents_of(id);
        }
        self.refresh_readiness();
        Ok(self.changes_since(&before))
    }

    fn task_mut(&mut self, id: &TaskId) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|task| task.id == *id)
    }

    fn states(&self) -> BTreeMap<TaskId, TaskState> {
        self.tasks
            .iter()
            .map(|task| (task.id.clone(), task.state))
            .collect()
    }

    fn changes_since(&self, before: &BTreeMap<TaskId, TaskState>) -> Completion {
        let mut completion = Completion::default();
        for task in &self.tasks {
            if before.get(&task.id) == Some(&task.state) {
                continue;
            }
            match task.state {
                TaskState::Ready => completion.now_ready.push(task.id.clone()),
                TaskState::Blocked => completion.now_blocked.push(task.id.clone()),
                _ => {}
            }
        }
        completion
    }

    fn unmet(&self, task: &Task) -> Vec<TaskId> {
        task.depends_on
            .iter()
            .filter(|dependency| {
                self.task(dependency)
                    .is_none_or(|dependency| dependency.state != TaskState::Done)
            })
            .cloned()
            .collect()
    }

    /// Marks everything downstream of a failure `Blocked`, transitively.
    /// A task already settled - dispatched, done, or failed in its own
    /// right - is left alone: what happened to it already happened.
    fn block_dependents_of(&mut self, failed: &TaskId) {
        let mut poisoned: BTreeSet<TaskId> = BTreeSet::new();
        poisoned.insert(failed.clone());
        // Repeat to a fixed point: `tasks` is in submission order, so one
        // pass can meet a dependent before the task that poisons it.
        loop {
            let newly: Vec<TaskId> =
                self.tasks
                    .iter()
                    .filter(|task| !poisoned.contains(&task.id))
                    .filter(|task| !task.state.is_settled())
                    .filter(|task| task.depends_on.iter().any(|d| poisoned.contains(d)))
                    .map(|task| task.id.clone())
                    .collect();
            if newly.is_empty() {
                break;
            }
            for id in newly {
                if let Some(task) = self.task_mut(&id) {
                    task.state = TaskState::Blocked;
                }
                poisoned.insert(id);
            }
        }
    }

    /// Promotes every unsettled task whose dependencies are all done, and
    /// demotes one whose are not. Settled tasks are never touched.
    fn refresh_readiness(&mut self) {
        let done: BTreeSet<TaskId> = self.tasks
                                         .iter()
                                         .filter(|task| task.state == TaskState::Done)
                                         .map(|task| task.id.clone())
                                         .collect();
        for task in &mut self.tasks {
            if task.state.is_settled() {
                continue;
            }
            task.state = if task.depends_on.iter().all(|d| done.contains(d)) {
                TaskState::Ready
            }
            else {
                TaskState::Pending
            };
        }
    }
}

/// Rejects a plan that cannot be run to completion, naming what is wrong.
fn validate(specs: &[TaskSpec]) -> Result<()> {
    if specs.len() > MAX_TASKS {
        return Err(TaskError::TooManyTasks { found: specs.len(),
                                             limit: MAX_TASKS, });
    }
    let mut seen: BTreeSet<&TaskId> = BTreeSet::new();
    for spec in specs {
        if !seen.insert(&spec.id) {
            return Err(TaskError::DuplicateId(spec.id.clone()));
        }
    }
    for spec in specs {
        for dependency in &spec.depends_on {
            if *dependency == spec.id {
                return Err(TaskError::SelfDependency(spec.id.clone()));
            }
            if !seen.contains(dependency) {
                return Err(TaskError::UnknownDependency { task:    spec.id.clone(),
                                                          missing: dependency.clone(), });
            }
        }
    }
    detect_cycle(specs)
}

/// Depth-first search reporting the cycle it walked into, not just that one
/// exists - a caller cannot fix "invalid graph".
fn detect_cycle(specs: &[TaskSpec]) -> Result<()> {
    let edges: BTreeMap<&TaskId, &Vec<TaskId>> = specs.iter()
                                                      .map(|spec| (&spec.id, &spec.depends_on))
                                                      .collect();
    let mut finished: BTreeSet<&TaskId> = BTreeSet::new();
    let mut path: Vec<&TaskId> = Vec::new();

    for spec in specs {
        if finished.contains(&spec.id) {
            continue;
        }
        if let Some(cycle) = walk(&spec.id, &edges, &mut finished, &mut path) {
            return Err(TaskError::Cycle(cycle));
        }
    }
    Ok(())
}

fn walk<'a>(id: &'a TaskId, edges: &BTreeMap<&'a TaskId, &'a Vec<TaskId>>,
            finished: &mut BTreeSet<&'a TaskId>, path: &mut Vec<&'a TaskId>)
            -> Option<Vec<TaskId>> {
    if let Some(start) = path.iter().position(|seen| *seen == id) {
        let mut cycle: Vec<TaskId> = path[start..].iter().map(|id| (*id).clone()).collect();
        cycle.push(id.clone());
        return Some(cycle);
    }
    if finished.contains(id) {
        return None;
    }
    path.push(id);
    if let Some(dependencies) = edges.get(id) {
        for dependency in dependencies.iter() {
            let key = edges.get_key_value(dependency).map(|(key, _)| *key);
            if let Some(key) = key
               && let Some(cycle) = walk(key, edges, finished, path)
            {
                return Some(cycle);
            }
        }
    }
    path.pop();
    finished.insert(id);
    None
}

#[cfg(test)]
mod tests;
