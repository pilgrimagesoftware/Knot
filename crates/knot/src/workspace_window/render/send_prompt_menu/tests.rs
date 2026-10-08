//! Unit tests for [`super`].

use super::*;

fn target(name: &str, enabled: bool) -> SendTarget {
    SendTarget { id: Uuid::new_v4(),
                 name: name.to_owned(),
                 enabled }
}

#[test]
fn the_agents_are_listed_alphabetically() {
    let names: Vec<_> = send_targets(vec![target("Bo", true),
                                          target("ada", true),
                                          target("Cy", true)]).into_iter()
                                                              .map(|target| target.name)
                                                              .collect();
    assert_eq!(names, ["ada", "Bo", "Cy"]);
}

/// A stopped agent stays in the list, disabled.
#[test]
fn a_stopped_agent_is_listed_disabled() {
    let targets = send_targets(vec![target("Bo", true), target("Ada", false)]);
    assert_eq!(targets.iter()
                      .map(|target| (target.name.as_str(), target.enabled))
                      .collect::<Vec<_>>(),
               [("Ada", false), ("Bo", true)]);
}

#[test]
fn each_prompt_names_its_item() {
    let url = "https://github.com/acme/widget/issues/42";
    let issue = WorkItemRef::Issue(url.to_owned()).prompt_text();
    assert!(issue.contains(url), "{issue}");
    assert!(!issue.contains("%{"), "{issue}");
    assert_eq!(issue,
               knot_core::l10n::t_with("changes_view.prompt.issue", &[("url", url)]));

    let change = WorkItemRef::Change("add-login".to_owned()).prompt_text();
    assert!(change.contains("add-login"), "{change}");
    assert!(!change.contains("%{"), "{change}");
}
