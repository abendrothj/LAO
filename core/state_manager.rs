use crate::workflow_state::{WorkflowState, WorkflowStatus};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct WorkflowStateManager {
    state_dir: PathBuf,
    states: HashMap<String, WorkflowState>,
}

impl WorkflowStateManager {
    pub fn new<P: AsRef<Path>>(state_dir: P) -> std::io::Result<Self> {
        let state_dir = state_dir.as_ref().to_path_buf();
        fs::create_dir_all(&state_dir)?;

        let mut manager = Self {
            state_dir,
            states: HashMap::new(),
        };

        manager.load_all_states()?;
        Ok(manager)
    }

    pub fn save_state(&mut self, state: &WorkflowState) -> std::io::Result<()> {
        let file_path = self.state_dir.join(format!("{}.json", state.workflow_id));
        let json = serde_json::to_string_pretty(state)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(file_path, json)?;
        self.states.insert(state.workflow_id.clone(), state.clone());
        Ok(())
    }

    pub fn load_state(&self, workflow_id: &str) -> std::io::Result<Option<WorkflowState>> {
        let file_path = self.state_dir.join(format!("{}.json", workflow_id));
        if !file_path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(file_path)?;
        let state: WorkflowState = serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(Some(state))
    }

    pub fn delete_state(&mut self, workflow_id: &str) -> std::io::Result<()> {
        let file_path = self.state_dir.join(format!("{}.json", workflow_id));
        if file_path.exists() {
            fs::remove_file(file_path)?;
        }
        self.states.remove(workflow_id);
        Ok(())
    }

    pub fn list_states(&self) -> Vec<&WorkflowState> {
        self.states.values().collect()
    }

    pub fn list_active_workflows(&self) -> Vec<&WorkflowState> {
        self.states
            .values()
            .filter(|state| {
                matches!(
                    state.status,
                    WorkflowStatus::Running | WorkflowStatus::Pending
                )
            })
            .collect()
    }

    pub fn list_scheduled_workflows(&self) -> Vec<&WorkflowState> {
        self.states
            .values()
            .filter(|state| matches!(state.status, WorkflowStatus::Scheduled))
            .collect()
    }

    fn load_all_states(&mut self) -> std::io::Result<()> {
        if !self.state_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&self.state_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(Some(state)) = self.load_state(filename) {
                        self.states.insert(state.workflow_id.clone(), state);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn cleanup_old_states(&mut self, max_age_hours: u64) -> std::io::Result<usize> {
        let cutoff =
            std::time::SystemTime::now() - std::time::Duration::from_secs(max_age_hours * 3600);
        let mut removed_count = 0;

        let to_remove: Vec<String> = self
            .states
            .values()
            .filter(|state| {
                matches!(
                    state.status,
                    WorkflowStatus::Completed | WorkflowStatus::Failed | WorkflowStatus::Cancelled
                ) && state
                    .completed_at
                    .map_or(false, |completed| completed < cutoff)
            })
            .map(|state| state.workflow_id.clone())
            .collect();

        for workflow_id in to_remove {
            self.delete_state(&workflow_id)?;
            removed_count += 1;
        }

        Ok(removed_count)
    }
}
