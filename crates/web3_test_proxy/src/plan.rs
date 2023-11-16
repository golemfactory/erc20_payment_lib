use crate::problems::ValuesChangeOptions;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemEntry {
    frames: Vec<u64>,
    keys: Vec<String>,
    values: ValuesChangeOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemProject {
    name: String,
    plan_type: String,
    pub frame_interval: f64,
    pub frame_cycle: Option<u64>,
    entries: Vec<ProblemEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortedProblemIterator {
    sorted_entries: Vec<SortedProblemEntry>,
    current_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortedProblemEntry {
    frame: u64,
    pub keys: Vec<String>,
    pub values: ValuesChangeOptions,
}

impl SortedProblemIterator {
    // sort problems by frame
    pub fn from_problem_project(problem_project: &ProblemProject) -> SortedProblemIterator {
        let sorted_entries: Vec<SortedProblemEntry> = problem_project
            .entries
            .iter()
            .flat_map(|entry| {
                // Use flat_map to handle nested structure
                entry.frames.iter().map(move |&frame| {
                    // Iterate over frames and map to SortedProblemEntry
                    SortedProblemEntry {
                        frame,
                        keys: entry.keys.clone(),
                        values: entry.values.clone(),
                    }
                })
            })
            .collect(); // Collect into a Vec

        // Sort the entries by frame.
        let mut sorted_entries = sorted_entries;
        sorted_entries.sort_by_key(|e| e.frame);

        let mut check_for_conflict = HashSet::<(String, u64)>::new();

        // Check for conflicts
        for entry in &sorted_entries {
            for key in &entry.keys {
                let key_frame = (key.clone(), entry.frame);
                if check_for_conflict.contains(&key_frame) {
                    panic!("Duplicate key frame: {:?}", key_frame);
                }
                check_for_conflict.insert(key_frame);
            }
        }

        SortedProblemIterator {
            sorted_entries,
            current_index: 0,
        }
    }

    pub fn get_next_entry(&mut self, current_frame: u64) -> Option<SortedProblemEntry> {
        if let Some(problem_entry) = self.sorted_entries.get(self.current_index) {
            if problem_entry.frame <= current_frame {
                self.current_index += 1;
                return Some(problem_entry.clone());
            }
            None
        } else {
            None
        }
    }
}
