//! Rendering a committed plan as a Mermaid flowchart.
//!
//! The panel reads `Agent::mermaid_source`, so this is how a plan reaches
//! the user: the same surface the `view-mermaid` tool already writes to,
//! rather than a second channel just for plans.
//!
//! # The dialect
//!
//! Deliberately a narrow subset, because `crates/knot/src/plan_view` parses
//! it back to draw the diagram:
//!
//! ```text
//! flowchart LR
//!   research["Research the API"]:::done
//!   build["Build it"]:::pending
//!   research --> build
//! ```
//!
//! One node line per task, carrying its id, its goal as the label and its
//! state as a class; one edge line per dependency, pointing from the
//! dependency to the task that waits on it. Nothing else is emitted, so the
//! reader stays small and total.

use knot_tasks::{Task, TaskGraph, TaskId};

/// Mermaid node ids are identifiers, task ids are whatever the orchestrator
/// typed. Anything outside the safe set becomes `_`.
fn sanitize_id(id: &TaskId) -> String {
    let sanitized: String = id.as_str()
                              .chars()
                              .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                              .collect();
    if sanitized.is_empty() {
        "task".to_string()
    }
    else {
        sanitized
    }
}

/// Labels are quoted, so a quote in a goal would end the label early.
/// Newlines would end the line. Both are replaced rather than escaped:
/// mermaid's escapes differ between renderers, and this text is read back
/// by one parser we own.
fn sanitize_label(goal: &str) -> String {
    goal.replace(['"', '\n', '\r'], " ").trim().to_string()
}

/// A plan as a Mermaid flowchart, left to right so dependencies read as
/// "what has to happen before this".
pub fn to_mermaid(graph: &TaskGraph) -> String {
    let mut out = String::from("flowchart LR\n");
    let ids: Vec<(String, &Task)> = graph.tasks()
                                         .iter()
                                         .map(|task| (unique_id(graph, task), task))
                                         .collect();
    for (id, task) in &ids {
        out.push_str(&format!("    {id}[\"{}\"]:::{}\n",
                              sanitize_label(&task.goal),
                              task.state.as_str()));
    }
    for (id, task) in &ids {
        for dependency in &task.depends_on {
            if let Some((from, _)) = ids.iter().find(|(_, other)| other.id == *dependency) {
                out.push_str(&format!("    {from} --> {id}\n"));
            }
        }
    }
    out
}

/// Sanitizing can make two distinct task ids collide (`a.b` and `a-b` both
/// become `a_b`), which would merge two tasks into one node. Disambiguate
/// by position, which is stable for a given plan.
fn unique_id(graph: &TaskGraph, task: &Task) -> String {
    let base = sanitize_id(&task.id);
    let collides = graph.tasks()
                        .iter()
                        .filter(|other| sanitize_id(&other.id) == base)
                        .count()
                   > 1;
    if !collides {
        return base;
    }
    let index = graph.tasks()
                     .iter()
                     .position(|other| other.id == task.id)
                     .unwrap_or_default();
    format!("{base}_{index}")
}

#[cfg(test)]
mod tests;
