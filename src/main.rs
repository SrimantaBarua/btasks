use std::net::SocketAddr;

use futures::TryStreamExt;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
enum State {
    Todo,
    InProgress,
    Blocked,
    Cancelled,
    Done,
}

#[derive(Serialize, Deserialize, Debug)]
enum LogEntryType {
    Opened,
    Comment(String),
    StateChangedTo(State),
}

#[derive(Serialize, Deserialize, Debug)]
struct LogEntry {
    #[serde(with = "chrono::serde::ts_seconds")]
    timestamp: chrono::DateTime<chrono::Utc>,
    entry_type: LogEntryType,
}

#[derive(Serialize, Deserialize, Debug)]
struct Task {
    title: String,
    description: String,
    id: usize,
    log: Vec<LogEntry>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Project {
    name: String,
    tasks: Vec<Task>,
}

struct AppState {}

impl AppState {
    fn initialize() -> AppState {
        AppState {}
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Could not set up Ctrl+C signal handler")
}

async fn echo(request: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let mut response = Response::new(Body::empty());
    match (request.method(), request.uri().path()) {
        (&Method::GET, "/") => *response.body_mut() = Body::from("Try POSTing data to /echo"),
        (&Method::POST, "/echo") => *response.body_mut() = request.into_body(),
        (&Method::POST, "/echo/reverse") => {
            let full_body = hyper::body::to_bytes(request.into_body()).await?;
            let reversed = full_body.iter().rev().cloned().collect::<Vec<_>>();
            *response.body_mut() = reversed.into();
        }
        (&Method::POST, "/echo/uppercase") => {
            let mapping = request.into_body().map_ok(|chunk| {
                chunk
                    .iter()
                    .map(|byte| byte.to_ascii_uppercase())
                    .collect::<Vec<_>>()
            });
            *response.body_mut() = Body::wrap_stream(mapping);
        }
        _ => *response.status_mut() = StatusCode::NOT_FOUND,
    }
    Ok(response)
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 12345));
    let make_echo_service =
        make_service_fn(|_conn| async { Ok::<_, hyper::Error>(service_fn(echo)) });
    let server = Server::bind(&addr)
        .serve(make_echo_service)
        .with_graceful_shutdown(shutdown_signal());
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
