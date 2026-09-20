use super::*;
use crate::jobs::JobStore;
use axum::{
    body::{Body, to_bytes},
    extract::FromRequest,
    http::Request,
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::sync::{Arc, Mutex};

async fn response_json(response: Response) -> Value {
    let body = to_bytes(response.into_body(), 4096).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

#[tokio::test]
async fn submission_stores_graph_and_status_lookup_omits_it() {
    let state: AppState = Arc::new(Mutex::new(JobStore::default()));
    let script = include_str!("../../ping.sh");
    let body = script.split_once("<<'JSON'\n").unwrap().1;
    let graph: Value = serde_json::from_str(body.split_once("\nJSON").unwrap().0).unwrap();
    let request = Request::builder()
        .method("POST")
        .uri("/jobs")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&graph).unwrap()))
        .unwrap();
    let request = Json::<CreateJobRequest>::from_request(request, &())
        .await
        .unwrap();
    let response = create_job(State(Arc::clone(&state)), request)
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let body = response_json(response).await;
    let job_id = body["job_id"].as_str().unwrap().to_owned();
    assert_eq!(body, json!({ "job_id": job_id, "status": "queued" }));

    {
        let store = state.lock().unwrap();
        assert_eq!(store.pending_job_ids.front(), Some(&job_id));
        let job = &store.jobs[&job_id];
        assert_eq!(
            job.graph.tasks().len(),
            graph["tasks"].as_array().unwrap().len()
        );
        for task in graph["tasks"].as_array().unwrap() {
            let id = task["id"].as_str().unwrap();
            assert_eq!(serde_json::to_value(&job.graph.tasks()[id]).unwrap(), *task);
        }
        assert_eq!(
            serde_json::to_value(job.graph.outputs()).unwrap(),
            graph["outputs"]
        );
    }

    let response = get_job_status(State(state), Path(job_id.clone()))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({ "job_id": job_id, "status": "queued", "result": null, "error": null })
    );
}

#[test]
fn arguments_support_arbitrary_json_literals_and_task_references() {
    use crate::graph::Argument;

    for argument in [
        json!({"value": null}),
        json!({"value": {"text": "hello", "options": [true, 3]}}),
        json!({"from_task": "upstream"}),
    ] {
        let parsed: Argument = serde_json::from_value(argument.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), argument);
    }
}

#[test]
fn malformed_graph_requests_are_rejected() {
    for argument in [
        json!({}),
        json!({"value": 1, "from_task": "upstream"}),
        json!({"from_task": 42}),
        json!({"unknown": "upstream"}),
        json!(42),
    ] {
        let request = json!({
            "tasks": [{"id": "a", "function": "f", "args": {"x": argument}}],
            "outputs": ["a"]
        });
        assert!(serde_json::from_value::<CreateJobRequest>(request).is_err());
    }

    for request in [
        json!({"task": "sum_numbers", "input": {"numbers": [1, 2]}}),
        json!({"tasks": []}),
        json!({"tasks": [], "outputs": [], "extra": true}),
        json!({"tasks": [{"id": "a", "function": "f"}], "outputs": ["a"]}),
        json!({"tasks": [{"id": "a", "function": "f", "args": {}, "extra": true}], "outputs": ["a"]}),
    ] {
        assert!(serde_json::from_value::<CreateJobRequest>(request).is_err());
    }
}

#[tokio::test]
async fn missing_job_returns_404_with_error_envelope() {
    let state = Arc::new(Mutex::new(JobStore::default()));
    let response = get_job_status(State(state), Path("missing".to_owned()))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response_json(response).await,
        json!({ "error": { "code": "JOB_NOT_FOUND", "message": "No job exists with that ID." } })
    );
}

#[tokio::test]
async fn poisoned_store_returns_500_instead_of_panicking() {
    let state = Arc::new(Mutex::new(JobStore::default()));
    let shared_state = Arc::clone(&state);
    let _ = std::thread::spawn(move || {
        let _guard = shared_state.lock().unwrap();
        panic!("simulate a failure while holding the store lock");
    })
    .join();

    let response = get_job_status(State(state), Path("any".to_owned()))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        response_json(response).await["error"]["code"],
        "INTERNAL_ERROR"
    );
}

#[tokio::test]
async fn invalid_submissions_return_422_without_storing_or_queueing() {
    let cases = [
        json!({"tasks": [], "outputs": ["a"]}),
        json!({"tasks": [{"id": "a", "function": "f", "args": {}}], "outputs": []}),
        json!({"tasks": [{"id": " ", "function": "f", "args": {}}], "outputs": [" "]}),
        json!({"tasks": [{"id": "a", "function": " ", "args": {}}], "outputs": ["a"]}),
        json!({"tasks": [{"id": "a", "function": "f", "args": {"x": {"from_task": "missing"}}}], "outputs": ["a"]}),
        json!({"tasks": [{"id": "a", "function": "f", "args": {}}], "outputs": ["missing"]}),
        json!({"tasks": [{"id": "a", "function": "f", "args": {"x": {"from_task": "a"}}}], "outputs": ["a"]}),
        json!({"tasks": [
            {"id": "a", "function": "first", "args": {}},
            {"id": "a", "function": "second", "args": {}}
        ], "outputs": ["a"]}),
    ];
    for submission in cases {
        let state: AppState = Arc::new(Mutex::new(JobStore::default()));
        let request = serde_json::from_value(submission.clone()).unwrap();
        let response = create_job(State(Arc::clone(&state)), Json(request))
            .await
            .into_response();
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{submission}"
        );
        let body = response_json(response).await;
        assert_eq!(body["error"]["code"], "INVALID_GRAPH", "{submission}");
        assert!(!body["error"]["message"].as_str().unwrap().is_empty());
        let store = state.lock().unwrap();
        assert!(store.jobs.is_empty(), "{submission}");
        assert!(store.pending_job_ids.is_empty(), "{submission}");
    }
}

#[tokio::test]
async fn invalid_graph_response_preserves_validation_message() {
    let response = ApiError::InvalidGraph("Unknown task: missing".into()).into_response();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        response_json(response).await,
        json!({
            "error": {"code": "INVALID_GRAPH", "message": "Unknown task: missing"}
        })
    );
}
