//! Unit tests for [`super`], pinning the scenarios in
//! `openspec/specs/agent-registry/spec.md`.

use knot_core::BenchAgent;

use super::*;
use crate::store::CreateOptions;

/// No adapter in these tests, so an entry's tools are just its type.
fn no_adapter_tools(_agent_type: &str) -> Vec<String> {
    Vec::new()
}

fn tagged(store: &mut AgentStore, name: &str, tags: &[&str], cost: CostTier,
          beside: Option<Uuid>)
          -> Uuid {
    store.create(format!("/repo/{name}"),
                 CreateOptions { name: Some(name.to_string()),
                                 capabilities: tags.iter().collect(),
                                 cost_tier: cost,
                                 insert_after: beside,
                                 ..Default::default() })
}

fn bench_entry(name: &str, tags: &[&str], cost: CostTier) -> BenchAgent {
    let mut bench = BenchAgent::new(Uuid::new_v4(), name, None, format!("/repo/{name}"));
    bench.capabilities = tags.iter().collect();
    bench.cost_tier = cost;
    bench
}

fn view<'a>(store: &'a AgentStore, bench: &'a [BenchAgent]) -> RegistryView<'a> {
    RegistryView::new(store, bench, &no_adapter_tools)
}

fn names(entries: &[RegistryEntry]) -> Vec<String> {
    entries.iter().map(|entry| entry.name.clone()).collect()
}

#[test]
fn a_bench_template_projects_with_no_live_status() {
    let mut store = AgentStore::new();
    let caller = tagged(&mut store, "caller", &[], CostTier::Medium, None);
    let bench = [bench_entry("Reviewer", &["code-review"], CostTier::Medium)];

    let entries = view(&store, &bench).candidates(caller, &RegistryQuery::everything());

    let reviewer = entries.iter().find(|e| e.name == "Reviewer").unwrap();
    assert_eq!(reviewer.status, RegistryStatus::Template);
    assert!(!reviewer.status.is_idle_agent());
}

#[test]
fn a_live_agent_projects_with_its_automatic_state() {
    let mut store = AgentStore::new();
    let caller = tagged(&mut store, "caller", &[], CostTier::Medium, None);
    let worker = tagged(&mut store,
                        "worker",
                        &["rust"],
                        CostTier::Medium,
                        Some(caller));
    store.agent_mut(worker).unwrap().state = AgentState::Running;
    store.agent_mut(worker).unwrap().status_text = "Refactoring auth".to_string();
    store.set_registered(worker, true);

    let entries = view(&store, &[]).candidates(caller, &RegistryQuery::everything());

    let entry = entries.iter().find(|e| e.name == "worker").unwrap();
    assert_eq!(entry.status,
               RegistryStatus::Live { state:         AgentState::Running,
                                      status_text:   "Refactoring auth".to_string(),
                                      is_registered: true, });
}

/// "Cheapest idle candidate ranks first".
#[test]
fn among_idle_candidates_the_cheaper_one_ranks_first() {
    let mut store = AgentStore::new();
    let caller = tagged(&mut store, "caller", &[], CostTier::Medium, None);
    tagged(&mut store,
           "expensive",
           &["code-review"],
           CostTier::High,
           Some(caller));
    tagged(&mut store,
           "cheap",
           &["code-review"],
           CostTier::Low,
           Some(caller));

    let query = RegistryQuery::for_capabilities(["code-review"].iter().collect());
    let entries = view(&store, &[]).candidates(caller, &query);

    assert_eq!(names(&entries), vec!["cheap", "expensive"]);
}

/// "A busy agent ranks below an idle one regardless of cost". Availability
/// outranks price, so a wrong cost tier costs at most a suboptimal pick.
#[test]
fn an_idle_expensive_agent_outranks_a_busy_cheap_one() {
    let mut store = AgentStore::new();
    let caller = tagged(&mut store, "caller", &[], CostTier::Medium, None);
    let busy = tagged(&mut store,
                      "cheap-but-busy",
                      &["rust"],
                      CostTier::Low,
                      Some(caller));
    store.agent_mut(busy).unwrap().state = AgentState::Running;
    tagged(&mut store,
           "costly-but-free",
           &["rust"],
           CostTier::High,
           Some(caller));

    let query = RegistryQuery::for_capabilities(["rust"].iter().collect());
    let entries = view(&store, &[]).candidates(caller, &query);

    assert_eq!(names(&entries), vec!["costly-but-free", "cheap-but-busy"]);
}

/// A template can always be deployed, but it has to be started first, so it
/// ranks behind every live agent however cheap it claims to be.
#[test]
fn a_template_ranks_behind_every_live_agent() {
    let mut store = AgentStore::new();
    let caller = tagged(&mut store, "caller", &[], CostTier::Medium, None);
    let busy = tagged(&mut store, "live", &["rust"], CostTier::High, Some(caller));
    store.agent_mut(busy).unwrap().state = AgentState::Running;
    let bench = [bench_entry("template", &["rust"], CostTier::Low)];

    let query = RegistryQuery::for_capabilities(["rust"].iter().collect());
    let entries = view(&store, &bench).candidates(caller, &query);

    assert_eq!(names(&entries), vec!["live", "template"]);
}

/// "All requested tags must match".
#[test]
fn a_candidate_must_carry_every_requested_tag() {
    let mut store = AgentStore::new();
    let caller = tagged(&mut store, "caller", &[], CostTier::Medium, None);
    tagged(&mut store,
           "rust-only",
           &["rust"],
           CostTier::Low,
           Some(caller));
    tagged(&mut store,
           "both",
           &["rust", "testing"],
           CostTier::High,
           Some(caller));

    let query = RegistryQuery::for_capabilities(["rust", "testing"].iter().collect());
    let entries = view(&store, &[]).candidates(caller, &query);

    assert_eq!(names(&entries), vec!["both"]);
}

/// "No candidate is an empty answer, not a failure".
#[test]
fn an_unmatched_tag_yields_an_empty_list() {
    let mut store = AgentStore::new();
    let caller = tagged(&mut store, "caller", &["rust"], CostTier::Medium, None);

    let query = RegistryQuery::for_capabilities(["code-reveiw"].iter().collect());
    let entries = view(&store, &[]).candidates(caller, &query);

    assert!(entries.is_empty());
}

/// "Companion visibility is unchanged".
#[test]
fn a_companion_the_caller_does_not_own_is_not_a_candidate() {
    let mut store = AgentStore::new();
    let caller = tagged(&mut store, "caller", &[], CostTier::Medium, None);
    let other = tagged(&mut store, "other", &[], CostTier::Medium, Some(caller));
    let companion = store.create_shell_companion(other).unwrap();
    store.agent_mut(companion).unwrap().capabilities = ["shell"].iter().collect();

    let query = RegistryQuery::for_capabilities(["shell"].iter().collect());
    let entries = view(&store, &[]).candidates(caller, &query);

    assert!(entries.is_empty(),
            "a companion belongs to its owner, not the workspace");

    let owned = view(&store, &[]).candidates(other, &query);
    assert_eq!(names(&owned).len(), 1, "its owner still sees it");
}

#[test]
fn an_agent_in_another_workspace_is_not_a_candidate() {
    let mut store = AgentStore::new();
    let caller = tagged(&mut store, "caller", &[], CostTier::Medium, None);
    let elsewhere = Uuid::new_v4();
    store.add_workspace(knot_core::Workspace { id:                    elsewhere,
                                               name:                  "Other".to_string(),
                                               color_hex:             "#000000".to_string(),
                                               agent_ids:             Vec::new(),
                                               active_agent_ids:      Vec::new(),
                                               layout_mode:           "single".to_string(),
                                               focused_pane_index:    0,
                                               split_ratio:           0.5,
                                               split_ratio_secondary: None,
                                               show_dashboard:        None,
                                               is_detached:           None,
                                               window_bounds:         None, });
    store.create("/repo/stranger",
                 CreateOptions { name: Some("stranger".to_string()),
                                 capabilities: ["rust"].iter().collect(),
                                 workspace_id: Some(elsewhere),
                                 ..Default::default() });

    let query = RegistryQuery::for_capabilities(["rust"].iter().collect());
    let entries = view(&store, &[]).candidates(caller, &query);

    assert!(entries.is_empty());
}

#[test]
fn templates_can_be_excluded_from_a_query() {
    let mut store = AgentStore::new();
    let caller = tagged(&mut store, "caller", &[], CostTier::Medium, None);
    let bench = [bench_entry("template", &["rust"], CostTier::Low)];
    let query = RegistryQuery { capabilities:      ["rust"].iter().collect(),
                                include_templates: false, };

    let entries = view(&store, &bench).candidates(caller, &query);

    assert!(entries.is_empty());
}

/// An empty tag set is how "list them all" is asked for.
#[test]
fn a_query_with_no_tags_returns_every_visible_entry() {
    let mut store = AgentStore::new();
    let caller = tagged(&mut store, "caller", &[], CostTier::Medium, None);
    tagged(&mut store, "worker", &["rust"], CostTier::Low, Some(caller));
    let bench = [bench_entry("template", &[], CostTier::Medium)];

    let entries = view(&store, &bench).candidates(caller, &RegistryQuery::everything());

    // Still ranked: `worker` is `Low` and `caller` is `Medium`, both idle,
    // so the cheaper one leads and the template brings up the rear.
    assert_eq!(names(&entries), vec!["worker", "caller", "template"]);
}

/// A query answer must be self-sufficient: everything needed to choose,
/// without a second call.
#[test]
fn an_entry_carries_everything_needed_to_choose_it() {
    let mut store = AgentStore::new();
    let caller = tagged(&mut store, "caller", &[], CostTier::Medium, None);
    let worker = tagged(&mut store,
                        "tester",
                        &["testing"],
                        CostTier::Low,
                        Some(caller));
    store.agent_mut(worker).unwrap().description = "Runs the test suite".to_string();

    let declared = |agent_type: &str| match agent_type {
        "claude" => vec!["Read".to_string(), "Bash".to_string()],
        _ => Vec::new(),
    };
    let entries = RegistryView::new(&store, &[], &declared)
        .candidates(caller, &RegistryQuery::for_capabilities(["testing"].iter().collect()));

    let entry = &entries[0];
    assert_eq!(entry.id, worker);
    assert_eq!(entry.description, "Runs the test suite");
    assert!(entry.capabilities.contains("testing"));
    assert_eq!(entry.cost_tier, CostTier::Low);
    assert!(entry.status.is_idle_agent());
    // The type always leads, with the adapter's declared surface after it.
    assert_eq!(entry.tools, vec!["claude", "Read", "Bash"]);
}

/// Equal availability and equal cost fall back to the name, so a listing is
/// stable rather than dependent on creation order.
#[test]
fn equally_ranked_candidates_are_ordered_by_name() {
    let mut store = AgentStore::new();
    let caller = tagged(&mut store, "caller", &[], CostTier::Medium, None);
    tagged(&mut store,
           "zeta",
           &["rust"],
           CostTier::Medium,
           Some(caller));
    tagged(&mut store,
           "alpha",
           &["rust"],
           CostTier::Medium,
           Some(caller));

    let query = RegistryQuery::for_capabilities(["rust"].iter().collect());
    let entries = view(&store, &[]).candidates(caller, &query);

    assert_eq!(names(&entries), vec!["alpha", "zeta"]);
}
