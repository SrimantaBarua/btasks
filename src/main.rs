use std::fs::File;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
enum State {
    Todo,
    InProgress,
    Blocked,
    Cancelled,
    Done,
}

#[derive(Serialize, Deserialize, Debug)]
enum LogEntryType {
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
    state: State,
}

#[derive(Serialize, Deserialize, Debug)]
struct Project {
    name: String,
    description: String,
    id: usize,
    tasks: Vec<Task>,
    next_task_id: usize,
}

#[derive(Default, Serialize, Deserialize, Debug)]
struct Database {
    projects: Vec<Project>,
    next_project_id: usize,
}

struct AppState {
    database: Database,
}

impl AppState {
    fn initialize() -> AppState {
        let database = Self::load_database().unwrap_or_default();
        println!("database: {:#?}", database);
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

#[derive(Serialize, Debug)]
struct ProjectPeek {
    id: usize,
    name: String,
}

async fn list_projects(
    _request: Request<Body>,
    app_state: Arc<Mutex<AppState>>,
) -> Result<Response<Body>, Box<dyn std::error::Error>> {
    let app = app_state.lock().unwrap();
    let projects = app
        .database
        .projects
        .iter()
        .map(|project| ProjectPeek {
            id: project.id,
            name: project.name.clone(),
        })
        .collect::<Vec<_>>();
    Ok(Response::new(Body::from(
        json!({ "projects": projects }).to_string(),
    )))
}

#[derive(Deserialize, Debug)]
struct ListTasksRequest {
    project_id: usize,
}

#[derive(Serialize, Debug)]
struct TaskPeek {
    id: usize,
    title: String,
    state: State,
}

async fn project_details(
    request: Request<Body>,
    app_state: Arc<Mutex<AppState>>,
) -> Result<Response<Body>, Box<dyn std::error::Error>> {
    let full_body = hyper::body::to_bytes(request.into_body()).await?;
    let list_tasks_request = serde_json::from_slice::<ListTasksRequest>(&full_body)?;
    let app = app_state.lock().unwrap();
    let project_index = app
        .database
        .projects
        .binary_search_by_key(&list_tasks_request.project_id, |project| project.id)
        .map_err(|_| {
            format!(
                "Could not find project with ID: {}",
                list_tasks_request.project_id
            )
        })?;
    let project = &app.database.projects[project_index];
    let tasks = project
        .tasks
        .iter()
        .map(|task| TaskPeek {
            id: task.id,
            title: task.title.clone(),
            state: task.state,
        })
        .collect::<Vec<_>>();
    Ok(Response::new(Body::from(
        json!({
            "name": project.name.clone(),
            "id": project.id,
            "description": project.description.clone(),
            "tasks": tasks
        }).to_string(),
    )))
}

fn wrap_error(
    inner: Result<Response<Body>, Box<dyn std::error::Error>>,
) -> Result<Response<Body>, hyper::Error> {
    match inner {
        Ok(response) => Ok(response),
        Err(error) => {
            let response_body = json!({
                "status": 500,
                "description": error.to_string(),
            })
            .to_string();
            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(response_body))
                .expect("Failed to build request"))
        }
    }
}

async fn request_handler(
    request: Request<Body>,
    app_state: Arc<Mutex<AppState>>,
) -> Result<Response<Body>, hyper::Error> {
    match (request.method(), request.uri().path()) {
        (&Method::GET, "/list_projects") => wrap_error(list_projects(request, app_state).await),
        (&Method::GET, "/project_details") => wrap_error(project_details(request, app_state).await),
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
