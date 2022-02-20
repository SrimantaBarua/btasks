use std::fs::File;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
enum State {
    Todo,
    InProgress,
    Blocked,
    Cancelled,
    Done,
}

#[derive(Serialize, Deserialize)]
enum LogEntryType {
    Opened,
    Comment(String),
    StateChangedTo(State),
}

#[derive(Serialize, Deserialize)]
struct LogEntry {
    #[serde(with = "chrono::serde::ts_seconds")]
    timestamp: chrono::DateTime<chrono::Utc>,
    entry_type: LogEntryType,
}

#[derive(Serialize, Deserialize)]
struct Task {
    title: String,
    description: String,
    log: Vec<LogEntry>,
}

#[derive(Serialize, Deserialize)]
struct Project {
    name: String,
    tasks: Vec<Task>,
}

#[derive(Default, Serialize, Deserialize)]
struct Database {
    projects: Vec<Project>,
}

struct AppState {
    database: Database,
}

impl AppState {
    fn initialize() -> AppState {
        let database = Self::load_database().unwrap_or_default();
        AppState { database }
    }

    fn load_database() -> Option<Database> {
        File::open(Self::get_database_path())
            .ok()
            .and_then(|file| serde_json::from_reader(file).ok())
    }

    fn get_database_path() -> PathBuf {
        let mut data_dir = dirs::data_dir().expect("Could not get data directory");
        data_dir.push("btasks");
        data_dir.push("database.json");
        eprintln!("Database path: {:?}", data_dir);
        data_dir
    }
}

async fn list_projects(
    _request: Request<Body>,
    app_state: Arc<Mutex<AppState>>,
) -> Result<Response<Body>, hyper::Error> {
    let app = app_state.lock().unwrap();
    let names = app
        .database
        .projects
        .iter()
        .enumerate()
        .map(|(index, project)| (index, project.name.clone()))
        .collect::<Vec<_>>();
    Ok(Response::new(Body::from(format!(
        "{{\"projects\":{}}}",
        serde_json::to_string(&names).expect("Could not format names")
    ))))
}

async fn request_handler(
    request: Request<Body>,
    app_state: Arc<Mutex<AppState>>,
) -> Result<Response<Body>, hyper::Error> {
    match (request.method(), request.uri().path()) {
        (&Method::GET, "/list_projects") => list_projects(request, app_state).await,
        _ => {
            let mut response = Response::new(Body::empty());
            *response.status_mut() = StatusCode::NOT_FOUND;
            Ok(response)
        }
    }
}

#[tokio::main]
async fn main() {
    let app_state = Arc::new(Mutex::new(AppState::initialize()));
    let addr = SocketAddr::from(([127, 0, 0, 1], 12345));
    let server = Server::bind(&addr)
        .serve(make_service_fn(move |_conn| {
            let app_state = app_state.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |request| {
                    request_handler(request, app_state.clone())
                }))
            }
        }))
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c()
                .await
                .expect("Could not set up Ctrl+C signal handler")
        });
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
