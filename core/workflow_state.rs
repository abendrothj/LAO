use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkflowState {
    pub workflow_id: String,
    pub workflow_name: String,
    pub status: WorkflowStatus,
    pub created_at: SystemTime,
    pub started_at: Option<SystemTime>,
    pub completed_at: Option<SystemTime>,
    pub current_step: usize,
    pub total_steps: usize,
    pub step_results: Vec<StepResult>,
    pub outputs: HashMap<String, String>,
    pub error_message: Option<String>,
    pub retry_count: u32,
    pub schedule: Option<WorkflowSchedule>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum WorkflowStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Scheduled,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StepResult {
    pub step_id: String,
    pub plugin_name: String,
    pub status: StepStatus,
    pub output: Option<String>,
    pub error: Option<String>,
    pub started_at: SystemTime,
    pub completed_at: Option<SystemTime>,
    pub duration_ms: Option<u64>,
    pub retry_count: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum StepStatus {
    Pending,
    Running,
    Success,
    Failed,
    Skipped,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkflowSchedule {
    pub cron_expression: Option<String>,
    pub next_run: Option<SystemTime>,
    pub enabled: bool,
    pub max_runs: Option<u32>,
    pub run_count: u32,
}

impl WorkflowState {
    pub fn new(workflow_id: String, workflow_name: String, total_steps: usize) -> Self {
        Self {
            workflow_id,
            workflow_name,
            status: WorkflowStatus::Pending,
            created_at: SystemTime::now(),
            started_at: None,
            completed_at: None,
            current_step: 0,
            total_steps,
            step_results: Vec::new(),
            outputs: HashMap::new(),
            error_message: None,
            retry_count: 0,
            schedule: None,
        }
    }

    pub fn start(&mut self) {
        self.status = WorkflowStatus::Running;
        self.started_at = Some(SystemTime::now());
    }

    pub fn complete(&mut self) {
        self.status = WorkflowStatus::Completed;
        self.completed_at = Some(SystemTime::now());
    }

    pub fn fail(&mut self, error: String) {
        self.status = WorkflowStatus::Failed;
        self.completed_at = Some(SystemTime::now());
        self.error_message = Some(error);
    }

    pub fn add_step_result(&mut self, result: StepResult) {
        self.step_results.push(result);
        self.current_step = self.step_results.len();
    }
}
