use crate::graph::TaskGraph;
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use crate::scheduler::Scheduler;

pub(crate) type AppState = Arc<Mutex<JobStore>>;

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum JobStatus {
    Queued,
}

pub(crate) struct Job {
    pub(crate) job_id: String,
    pub(crate) graph: TaskGraph,
    pub(crate) status: JobStatus,
    pub(crate) result: Option<Value>,
    pub(crate) error: Option<TaskError>,
	pub(crate) scheduler: Scheduler,
}

#[derive(Default)]
pub(crate) struct JobStore {
    pub(crate) jobs: HashMap<String, Job>,
    pub(crate) pending_job_ids: VecDeque<String>,
}

#[derive(Clone, Serialize)]
pub(crate) struct TaskError {
    pub(crate) code: String,
    pub(crate) message: String,
}
