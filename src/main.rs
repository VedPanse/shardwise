mod api;
mod graph;
mod jobs;
mod scheduler;

use jobs::{AppState, JobStore};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let state: AppState = Arc::new(Mutex::new(JobStore::default()));
    let app = api::router(state);

    let address = SocketAddr::from(([127, 0, 0, 1], 8080));
    let listener = tokio::net::TcpListener::bind(address).await?;
    println!("Serving on http://{address}");
    axum::serve(listener, app).await
}
