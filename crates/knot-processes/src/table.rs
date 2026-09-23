//! Indexes a parsed process table by PID and answers the one question the
//! capability asks of it: what descends from an agent's session root, and
//! which of those descendants are running in the background.
//!
//! One table serves every observed root in a window. `ps -A` already returns
//! the whole machine, so a second expanded section costs a walk over data
//! already in memory rather than a second read.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::str::FromStr;
use std::time::Duration;

use crate::record::ProcessRecord;

/// Whether a descendant is running in its terminal's foreground process group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Activity {
    /// Outside the terminal's foreground process group, or with no controlling
    /// terminal at all -- the whole of an ACP adapter's subtree.
    Background,
    /// The command the agent's terminal is currently running.
    Foreground,
}

impl Activity {
    pub fn is_background(self) -> bool {
        matches!(self, Self::Background)
    }

    /// The stable key this variant is named by, in localization and in tests.
    pub fn key(self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::Foreground => "foreground",
        }
    }
}

impl fmt::Display for Activity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key())
    }
}

impl FromStr for Activity {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "background" => Ok(Self::Background),
            "foreground" => Ok(Self::Foreground),
            _ => Err(()),
        }
    }
}

/// One live process descending from an agent's session root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescendantProcess {
    pub pid:      u32,
    pub ppid:     u32,
    pub command:  String,
    pub elapsed:  Duration,
    pub activity: Activity,
}

impl DescendantProcess {
    fn from_record(record: &ProcessRecord) -> Self {
        let activity = if record.is_foreground() {
            Activity::Foreground
        }
        else {
            Activity::Background
        };

        Self { pid: record.pid,
               ppid: record.ppid,
               command: record.command.clone(),
               elapsed: record.elapsed,
               activity }
    }
}

/// A whole process table, indexed by PID.
#[derive(Debug, Clone, Default)]
pub struct ProcessTable {
    by_pid: HashMap<u32, ProcessRecord>,
}

impl ProcessTable {
    pub fn from_records(records: impl IntoIterator<Item = ProcessRecord>) -> Self {
        Self { by_pid: records.into_iter()
                              .map(|record| (record.pid, record))
                              .collect(), }
    }

    pub fn get(&self, pid: u32) -> Option<&ProcessRecord> {
        self.by_pid.get(&pid)
    }

    pub fn contains(&self, pid: u32) -> bool {
        self.by_pid.contains_key(&pid)
    }

    pub fn len(&self) -> usize {
        self.by_pid.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_pid.is_empty()
    }

    /// Every live process transitively descending from `root`, to unlimited
    /// depth, excluding `root` itself.
    ///
    /// Ordered for display: background before foreground, longest-running
    /// first within each group. An unknown `root` yields an empty list, which
    /// is the same answer as a root with no children -- the caller decides
    /// which case it is from whether the agent is running.
    pub fn descendants(&self, root: u32) -> Vec<DescendantProcess> {
        let children = self.children_by_parent();
        let mut found = Vec::new();
        let mut seen = HashSet::from([root]);
        let mut queue = vec![root];

        // Breadth-first over the child index, with a visited set: a parent
        // chain that loops back on itself -- which the table can express even
        // though the kernel cannot -- terminates instead of spinning.
        while let Some(pid) = queue.pop() {
            let Some(kids) = children.get(&pid)
            else {
                continue;
            };

            for &kid in kids {
                if !seen.insert(kid) {
                    continue;
                }

                queue.push(kid);

                if let Some(record) = self.by_pid.get(&kid) {
                    found.push(DescendantProcess::from_record(record));
                }
            }
        }

        found.sort_by(|left, right| {
                 left.activity
                     .cmp(&right.activity)
                     .then(right.elapsed.cmp(&left.elapsed))
                     .then(left.pid.cmp(&right.pid))
             });

        found
    }

    /// Whether `pid` is a descendant of `root` in this table.
    pub fn is_descendant_of(&self, pid: u32, root: u32) -> bool {
        let mut seen = HashSet::new();
        let mut current = pid;

        while current != 0 && seen.insert(current) {
            let Some(record) = self.by_pid.get(&current)
            else {
                return false;
            };

            if record.ppid == root {
                return true;
            }

            current = record.ppid;
        }

        false
    }

    fn children_by_parent(&self) -> HashMap<u32, Vec<u32>> {
        let mut children: HashMap<u32, Vec<u32>> = HashMap::new();

        for record in self.by_pid.values() {
            // A process that is its own parent would make the walk's visited
            // set the only thing standing between it and a loop; drop the edge
            // instead.
            if record.ppid != record.pid {
                children.entry(record.ppid).or_default().push(record.pid);
            }
        }

        children
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Activity, ProcessTable};
    use crate::record::ProcessRecord;

    fn record(pid: u32, ppid: u32, command: &str) -> ProcessRecord {
        ProcessRecord { pid,
                        ppid,
                        pgid: pid,
                        tpgid: 0,
                        elapsed: Duration::from_secs(u64::from(pid)),
                        command: command.to_owned() }
    }

    fn foreground(pid: u32, ppid: u32, group: u32, command: &str) -> ProcessRecord {
        ProcessRecord { pid,
                        ppid,
                        pgid: group,
                        tpgid: group as i32,
                        elapsed: Duration::from_secs(u64::from(pid)),
                        command: command.to_owned() }
    }

    #[test]
    fn reports_a_grandchild() {
        let table = ProcessTable::from_records([record(100, 1, "shell"),
                                                record(200, 100, "npm run dev"),
                                                record(300, 200, "node server.js")]);

        let pids: Vec<_> = table.descendants(100).iter().map(|p| p.pid).collect();

        assert_eq!(pids.len(), 2);
        assert!(pids.contains(&200));
        assert!(pids.contains(&300));
    }

    #[test]
    fn excludes_the_root_itself() {
        let table =
            ProcessTable::from_records([record(100, 1, "shell"), record(200, 100, "child")]);

        assert!(table.descendants(100).iter().all(|p| p.pid != 100));
    }

    #[test]
    fn excludes_an_unrelated_process_with_a_recycled_looking_pid() {
        let table = ProcessTable::from_records([record(100, 1, "shell"),
                                                record(200, 100, "child"),
                                                record(201, 1, "unrelated")]);

        let pids: Vec<_> = table.descendants(100).iter().map(|p| p.pid).collect();

        assert_eq!(pids, vec![200]);
    }

    #[test]
    fn a_cycle_in_the_parent_chain_terminates() {
        // Two processes each claiming the other as parent: impossible from the
        // kernel, expressible in a table assembled from two moments of one.
        let table = ProcessTable::from_records([record(100, 1, "shell"),
                                                record(200, 100, "child"),
                                                record(300, 400, "looping"),
                                                record(400, 300, "looping too")]);

        assert_eq!(table.descendants(100).len(), 1);
        assert!(table.descendants(300).iter().any(|p| p.pid == 400));
    }

    #[test]
    fn a_self_parenting_process_does_not_loop() {
        let table = ProcessTable::from_records([record(100, 100, "its own parent")]);

        assert!(table.descendants(100).is_empty());
    }

    #[test]
    fn an_unknown_root_has_no_descendants() {
        let table = ProcessTable::from_records([record(100, 1, "shell")]);

        assert!(table.descendants(9_999).is_empty());
    }

    #[test]
    fn classifies_a_foreground_command_and_a_detached_process() {
        let table = ProcessTable::from_records([foreground(100, 1, 100, "shell"),
                                                foreground(200, 100, 100, "cargo test"),
                                                record(300, 100, "node server.js &")]);

        let descendants = table.descendants(100);

        let cargo = descendants.iter().find(|p| p.pid == 200).unwrap();
        assert_eq!(cargo.activity, Activity::Foreground);

        let server = descendants.iter().find(|p| p.pid == 300).unwrap();
        assert_eq!(server.activity, Activity::Background);
    }

    #[test]
    fn a_subtree_with_no_controlling_terminal_is_all_background() {
        let table = ProcessTable::from_records([record(100, 1, "acp-adapter"),
                                                record(200, 100, "child"),
                                                record(300, 200, "grandchild")]);

        assert!(table.descendants(100)
                     .iter()
                     .all(|p| p.activity == Activity::Background));
    }

    #[test]
    fn orders_background_first_then_longest_running() {
        let table = ProcessTable::from_records([foreground(100, 1, 100, "shell"),
                                                foreground(400, 100, 100, "foreground"),
                                                record(200, 100, "short background"),
                                                record(300, 100, "long background")]);

        let order: Vec<_> = table.descendants(100).iter().map(|p| p.pid).collect();

        // 300 and 200 are background, 300 has the larger elapsed; 400 is
        // foreground and sorts last regardless of runtime.
        assert_eq!(order, vec![300, 200, 400]);
    }

    #[test]
    fn descendant_membership_is_checked_through_the_parent_chain() {
        let table = ProcessTable::from_records([record(100, 1, "shell"),
                                                record(200, 100, "child"),
                                                record(300, 200, "grandchild"),
                                                record(400, 1, "unrelated")]);

        assert!(table.is_descendant_of(200, 100));
        assert!(table.is_descendant_of(300, 100));
        assert!(!table.is_descendant_of(400, 100));
        assert!(!table.is_descendant_of(100, 100),
                "the root is not its own descendant");
        assert!(!table.is_descendant_of(9_999, 100));
    }

    #[test]
    fn activity_round_trips_through_its_key() {
        for activity in [Activity::Background, Activity::Foreground] {
            assert_eq!(activity.to_string().parse(), Ok(activity));
        }

        assert_eq!("sideways".parse::<Activity>(), Err(()));
    }
}
