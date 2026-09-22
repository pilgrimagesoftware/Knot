//! Reading back the Mermaid flowchart subset the task tools emit.
//!
//! The panel is handed a string, not a graph - `Agent::mermaid_source` is
//! the surface `view-mermaid` already writes to, and reusing it is what
//! lets any agent's diagram render, not just an orchestrator's plan. So the
//! structure has to be recovered from the text.
//!
//! Deliberately partial and total: it understands the dialect
//! `knot_mcp_tools`'s plan renderer emits (see its `tasks::mermaid`), and
//! ignores every line it does not, rather than failing. A diagram from some
//! other agent's `view-mermaid` call is then drawn as far as it is
//! understood instead of showing nothing.

/// A node's state class, if it carried one. Unknown classes are kept as
/// written: the renderer colours the ones it knows and leaves the rest
/// neutral, so a future state does not have to be taught here first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanNode {
    pub id:    String,
    pub label: String,
    pub state: Option<String>,
}

/// A parsed diagram: nodes in declaration order, edges as index pairs into
/// them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlanGraph {
    pub nodes: Vec<PlanNode>,
    pub edges: Vec<(usize, usize)>,
}

impl PlanGraph {
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    fn index_of(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|node| node.id == id)
    }
}

/// Parses a node line: `id["label"]` with an optional `:::state`.
fn parse_node(line: &str) -> Option<PlanNode> {
    let (id, rest) = line.split_once("[\"")?;
    let id = id.trim();
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    let (label, rest) = rest.split_once("\"]")?;
    let state = rest.trim()
                    .strip_prefix(":::")
                    .map(str::trim)
                    .filter(|state| !state.is_empty())
                    .map(str::to_string);
    Some(PlanNode { id: id.to_string(),
                    label: label.to_string(),
                    state })
}

/// Parses an edge line: `from --> to`.
fn parse_edge(line: &str) -> Option<(String, String)> {
    let (from, to) = line.split_once("-->")?;
    let (from, to) = (from.trim(), to.trim());
    if from.is_empty() || to.is_empty() {
        return None;
    }
    Some((from.to_string(), to.to_string()))
}

/// Recovers what it can from `source`. Never fails; an unrecognised
/// document yields an empty graph, which the caller shows as "nothing to
/// draw" rather than as an error.
pub fn parse(source: &str) -> PlanGraph {
    let mut graph = PlanGraph::default();
    let mut edges: Vec<(String, String)> = Vec::new();

    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        // Edges first: a node line cannot contain `-->`, but an edge line
        // could otherwise be mistaken for a malformed node.
        if let Some(edge) = parse_edge(line) {
            edges.push(edge);
        }
        else if let Some(node) = parse_node(line) {
            graph.nodes.push(node);
        }
    }
    // Resolved after every node is known, so an edge declared before its
    // target still connects.
    for (from, to) in edges {
        if let (Some(from), Some(to)) = (graph.index_of(&from), graph.index_of(&to))
           && from != to
           && !graph.edges.contains(&(from, to))
        {
            graph.edges.push((from, to));
        }
    }
    graph
}

#[cfg(test)]
mod tests;
