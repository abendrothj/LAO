use crate::state_manager::WorkflowStateManager;
use crate::workflow_state::{WorkflowSchedule, WorkflowState, WorkflowStatus};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

pub struct WorkflowScheduler {
    state_manager: WorkflowStateManager,
    scheduled_workflows: HashMap<String, ScheduledWorkflow>,
}

#[derive(Debug, Clone)]
pub struct ScheduledWorkflow {
    pub workflow_path: String,
    pub schedule: WorkflowSchedule,
    pub last_run: Option<SystemTime>,
    pub next_run: SystemTime,
}

impl WorkflowScheduler {
    pub fn new(state_dir: &str) -> std::io::Result<Self> {
        let state_manager = WorkflowStateManager::new(state_dir)?;
        Ok(Self {
            state_manager,
            scheduled_workflows: HashMap::new(),
        })
    }

    pub fn schedule_workflow(
        &mut self,
        workflow_id: String,
        workflow_path: String,
        schedule: WorkflowSchedule,
    ) -> Result<(), String> {
        let next_run = self.calculate_next_run(&schedule)?;

        let scheduled = ScheduledWorkflow {
            workflow_path,
            schedule: schedule.clone(),
            last_run: None,
            next_run,
        };

        self.scheduled_workflows
            .insert(workflow_id.clone(), scheduled);

        // Create a scheduled workflow state
        let mut state = WorkflowState::new(workflow_id, "Scheduled Workflow".to_string(), 0);
        state.status = WorkflowStatus::Scheduled;
        state.schedule = Some(schedule);

        self.state_manager
            .save_state(&state)
            .map_err(|e| format!("Failed to save scheduled workflow state: {}", e))?;

        Ok(())
    }

    pub fn unschedule_workflow(&mut self, workflow_id: &str) -> Result<(), String> {
        self.scheduled_workflows.remove(workflow_id);
        self.state_manager
            .delete_state(workflow_id)
            .map_err(|e| format!("Failed to delete workflow state: {}", e))?;
        Ok(())
    }

    pub fn get_due_workflows(&self) -> Vec<String> {
        let now = SystemTime::now();
        self.scheduled_workflows
            .iter()
            .filter(|(_, scheduled)| scheduled.next_run <= now && scheduled.schedule.enabled)
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn update_workflow_run(&mut self, workflow_id: &str) -> Result<(), String> {
        // Extract schedule info to avoid borrowing conflicts
        let (schedule, max_runs) = {
            if let Some(scheduled) = self.scheduled_workflows.get(workflow_id) {
                (scheduled.schedule.clone(), scheduled.schedule.max_runs)
            } else {
                return Ok(());
            }
        };

        // Calculate next run time
        let next_run = self.calculate_next_run(&schedule)?;

        // Update the scheduled workflow
        if let Some(scheduled) = self.scheduled_workflows.get_mut(workflow_id) {
            scheduled.last_run = Some(SystemTime::now());
            scheduled.next_run = next_run;
            scheduled.schedule.run_count += 1;

            // Check if max runs reached
            if let Some(max_runs) = max_runs {
                if scheduled.schedule.run_count >= max_runs {
                    scheduled.schedule.enabled = false;
                }
            }
        }
        Ok(())
    }

    pub fn list_scheduled_workflows(&self) -> Vec<(String, &ScheduledWorkflow)> {
        self.scheduled_workflows
            .iter()
            .map(|(id, scheduled)| (id.clone(), scheduled))
            .collect()
    }

    fn calculate_next_run(&self, schedule: &WorkflowSchedule) -> Result<SystemTime, String> {
        if let Some(cron_expr) = &schedule.cron_expression {
            // For now, implement simple interval parsing
            // In a full implementation, you'd use a cron parsing library
            self.parse_simple_cron(cron_expr)
        } else {
            // Default to 1 hour from now
            Ok(SystemTime::now() + Duration::from_secs(3600))
        }
    }

    fn parse_simple_cron(&self, cron_expr: &str) -> Result<SystemTime, String> {
        // Simple cron parser for common patterns
        // Format: "interval:minutes" or "daily:HH:MM" or "weekly:day:HH:MM"
        let parts: Vec<&str> = cron_expr.split(':').collect();

        match parts.as_slice() {
            ["interval", minutes_str] => {
                let minutes: u64 = minutes_str
                    .parse()
                    .map_err(|_| format!("Invalid interval minutes: {}", minutes_str))?;
                Ok(SystemTime::now() + Duration::from_secs(minutes * 60))
            }
            ["daily", hour_str, minute_str] => {
                let _hour: u32 = hour_str
                    .parse()
                    .map_err(|_| format!("Invalid hour: {}", hour_str))?;
                let _minute: u32 = minute_str
                    .parse()
                    .map_err(|_| format!("Invalid minute: {}", minute_str))?;
                // Simplified: schedule for next day at same time
                Ok(SystemTime::now() + Duration::from_secs(24 * 3600))
            }
            ["weekly", _day, _hour, _minute] => {
                // Simplified: schedule for next week
                Ok(SystemTime::now() + Duration::from_secs(7 * 24 * 3600))
            }
            _ => Err(format!("Invalid cron expression format: {}", cron_expr)),
        }
    }

    pub fn cleanup_old_states(&mut self, max_age_hours: u64) -> std::io::Result<usize> {
        self.state_manager.cleanup_old_states(max_age_hours)
    }

    pub fn get_workflow_history(
        &self,
        workflow_id: &str,
    ) -> std::io::Result<Option<WorkflowState>> {
        self.state_manager.load_state(workflow_id)
    }

    pub fn list_workflow_states(&self) -> Vec<&WorkflowState> {
        self.state_manager.list_states()
    }
}
