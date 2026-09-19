use axum::{Json, Router, routing::{get, post}, extract::{Path, State}};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::net::SocketAddr;
use uuid::Uuid;
use std::sync::{Arc, Mutex};
use std::collections::{HashMap, VecDeque};

type AppState = Arc<Mutex<JobStore>>;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
enum Status {
	Queued, Running, Succeeded(String), Failed(String),
}

#[derive(Serialize, Debug, Clone)]
enum JobSuccessStatus {
	Normal(Job), Error(JobError)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateJobRequest {
    task: String,
    input: Map<String, Value>,
}

#[derive(Serialize)]
struct CreateJobResponse {
    job_id: String,
    status: Status,
}

#[derive(Serialize)]
struct HealthStatusResponse {
	status: String,
}

#[derive(Serialize, Debug, Clone)]
struct Job {
	job_id: String,
	task: String,
	input: Map<String, Value>,
	status: Status,
	result: Option<Value>,
	error: Option<JobError>,
}

struct JobStore {
	jobs: HashMap<String, Job>,
	pending: VecDeque<String>,
}

#[derive(Serialize, Debug, Clone)]
struct JobError {
	code: String,
	message: String,
}

async fn create_job(State(state): State<AppState>, Json(payload): Json<CreateJobRequest>) -> Json<CreateJobResponse> {
    println!(
        "Received task {} with {} input fields",
        payload.task,
        payload.input.len()
    );
	
	let job = Job {
		job_id: Uuid::now_v7().to_string(),
		task: payload.task,
		input: payload.input,
		status: Status::Queued,
		result: None,
		error: None,
	};
	
    let response = CreateJobResponse {
        job_id: job.job_id.clone(),
        status: Status::Queued,
    };
    let stored_job_id = job.job_id.clone();
    let queued_job_id = job.job_id.clone();

    {
        let mut store = state.lock().unwrap();
        store.jobs.insert(stored_job_id, job);
        store.pending.push_back(queued_job_id);
    }

    Json(response)
}

async fn get_job_status(State(state): State<AppState>, Path(job_id): Path<String>) -> Json<JobSuccessStatus> {	
	let store = state.lock().unwrap();
	
	match store.jobs.get(&job_id) {
		Some(job) => Json(JobSuccessStatus::Normal(job.clone())),
		None => Json(JobSuccessStatus::Error(JobError {
			code: "JOB_NOT_FOUND".to_string(),
			message: format!("No job exists with id: {}", job_id).to_string(),
			}))
	}
}

async fn get_health() -> Json<HealthStatusResponse> {
	let response = HealthStatusResponse {
		status: "ok".to_string(),
	};
	
	Json(response)
}

#[tokio::main]
async fn main() {
	let state: AppState = Arc::new(Mutex::new(JobStore {
		jobs: HashMap::new(),
		pending: VecDeque::new(),
	}));
	
    let app = Router::new().route("/jobs", post(create_job))
						   .route("/jobs/{job_id}", get(get_job_status))
						   .route("/health", get(get_health))
						   .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    println!("Serving on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
