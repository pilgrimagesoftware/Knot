//! Probes the agent CLIs actually installed on this machine.
//!
//! Ignored by default and never run in CI: it needs a real `claude` or
//! `gemini` on `PATH`, with whatever MCP servers that user happens to have
//! configured, and it goes over the network to health-check them. A test
//! whose result depends on all of that is not one to gate a merge on.
//!
//! It exists because the fixtures beside the parser are captured output, and
//! captured output goes stale silently. This is the check that says the
//! format has not moved under us:
//!
//! ```text
//! cargo test -p knot-mcp-probe --test against_a_real_cli -- --ignored --nocapture
//! ```
//!
//! What to look for: every server classified as something other than
//! `Unknown`, and in particular a server disabled for the project reading as
//! `Disabled` rather than `Failed`. A wall of `Unknown` means the CLI's
//! output shape has changed and the fixtures need recapturing.

use std::path::Path;

use knot_mcp_probe::{CommandRunner, Inventory, ProbePlan, ServerState, probe};

/// `report`'s sanity check assumes most rows classify as something other
/// than `Unknown`, which holds for a CLI that health-checks as it lists.
/// Codex's listing is configuration, not a probe (see
/// `knot_mcp_probe::parse::codex`), so an *enabled* row is `Unknown` by
/// design and that assumption does not apply to it.
fn codex_classifies_disabled_rows_correctly() {
    let plan = knot_mcp_probe::plan_for("codex",
                                        Path::new("."),
                                        vec![("PATH".to_owned(),
                                              knot_core::exec_path::search_path())],
                                        None);

    if matches!(plan, ProbePlan::Unsupported) {
        println!("\ncodex: no reader - nothing to check");
        return;
    }

    println!("\n=== codex ===");

    match probe(&CommandRunner::new(), &plan) {
        Err(error) => println!("  probe failed: {error}"),
        Ok(Inventory::Unprobeable) => println!("  unprobeable"),
        Ok(inventory) => {
            for row in inventory.rows() {
                println!("  {:<34} {:<6} {}", row.name, row.target.token(), row.state);
                assert_ne!(row.state,
                           ServerState::Connected,
                           "{} read as Connected - Codex's listing never probes a connection",
                           row.name);
            }
        }
    }
}

fn report(agent_type: &str) {
    let plan = knot_mcp_probe::plan_for(agent_type,
                                        Path::new("."),
                                        vec![("PATH".to_owned(),
                                              knot_core::exec_path::search_path())],
                                        None);

    if matches!(plan, ProbePlan::Unsupported) {
        println!("\n{agent_type}: no reader - nothing to check");
        return;
    }

    println!("\n=== {agent_type} ===");

    match probe(&CommandRunner::new(), &plan) {
        Err(error) => println!("  probe failed: {error}"),
        Ok(Inventory::Unprobeable) => println!("  unprobeable"),
        Ok(inventory) => {
            if inventory.found_none() {
                println!("  no MCP servers configured");
                return;
            }

            let unknown = inventory.rows()
                                   .iter()
                                   .filter(|row| row.state == ServerState::Unknown)
                                   .count();

            for row in inventory.rows() {
                println!("  {:<34} {:<6} {}", row.name, row.target.token(), row.state);
            }

            println!("  -> {} rows, {} needing attention, {unknown} unclassified",
                     inventory.rows().len(),
                     inventory.attention_count());

            assert!(unknown * 2 <= inventory.rows().len(),
                    "{unknown} of {} rows were unclassified - {agent_type}'s output format has \
                     probably moved, and the fixtures need recapturing",
                    inventory.rows().len());
        }
    }
}

#[test]
#[ignore = "needs a real agent CLI installed, and goes over the network"]
fn the_installed_clis_still_parse() {
    report("claude");
    report("gemini");
    codex_classifies_disabled_rows_correctly();
}
