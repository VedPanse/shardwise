mod error;
#[cfg(test)]
mod tests;

use self::error::ApiError;
use crate::graph::{TaskDefinition, TaskGraph};
use crate::jobs::{AppState, Job, JobStatus, TaskError};
use crate::scheduler::Scheduler;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateJobRequest {
    tasks: Vec<TaskDefinition>,
    outputs: Vec<String>,
}

#[derive(Serialize)]
struct CreateJobResponse {
    job_id: String,
    status: JobStatus,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize)]
struct JobResponse {
    job_id: String,
    status: JobStatus,
    result: Option<Value>,
    error: Option<TaskError>,
}

impl From<&Job> for JobResponse {
    fn from(job: &Job) -> Self {
        Self {
            job_id: job.job_id.clone(),
            status: job.status,
            result: job.result.clone(),
            error: job.error.clone(),
        }
    }
}

pub(crate) fn router(state: AppState) -> Router {
    Router::new()
        .route("/jobs", post(create_job))
        .route("/jobs/{job_id}", get(get_job_status))
        .route("/health", get(get_health))
        .with_state(state)
}

async fn create_job(
    State(state): State<AppState>,
    Json(request): Json<CreateJobRequest>,
) -> Result<(StatusCode, Json<CreateJobResponse>), ApiError> {
    let graph = TaskGraph::new(request.tasks, request.outputs)
						   .map_err(ApiError::InvalidGraph)?;
		
	graph.validate().map_err(ApiError::InvalidGraph)?;
	
	let scheduler = Scheduler::new(&graph);
		
    let job = Job {
        job_id: Uuid::now_v7().to_string(),
        graph,
        status: JobStatus::Queued,
        result: None,
        error: None,
		scheduler,
    };

    println!(
        "Received job {} with {} tasks and {} requested outputs",
        job.job_id,
        job.graph.tasks().len(),
        job.graph.outputs().len()
    );

    print!("{}", job.graph.draw_graph());

    let response = CreateJobResponse {
        job_id: job.job_id.clone(),
        status: job.status,
    };
    let map_key = job.job_id.clone();
    let queued_job_id = job.job_id.clone();

    {
        let mut store = state.lock().map_err(|_| ApiError::StoreUnavailable)?;
        store.jobs.insert(map_key, job);
        store.pending_job_ids.push_back(queued_job_id);
    }

    Ok((StatusCode::ACCEPTED, Json(response)))
}

async fn get_job_status(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<JobResponse>, ApiError> {
    let response = {
        let store = state.lock().map_err(|_| ApiError::StoreUnavailable)?;
        let job = store.jobs.get(&job_id).ok_or(ApiError::JobNotFound)?;
        JobResponse::from(job)
    };

    Ok(Json(response))
}

async fn get_health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}
