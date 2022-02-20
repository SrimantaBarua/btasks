use std::fs::File;
use std::net::SocketAddr;
use std::os::unix::fs::chroot;
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

impl Task {
    fn new_log_entry(&mut self, entry_type: LogEntryType) {
        let timestamp = chrono::Utc::now();
        self.log.push(LogEntry {
            timestamp,
            entry_type,
        });
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Project {
    name: String,
    description: String,
    id: usize,
    tasks: Vec<Task>,
    next_task_id: usize,
}

impl Project {
    fn find_task_by_id(&self, id: usize) -> Result<&Task, Box<dyn std::error::Error>> {
        let task_index = self
            .tasks
            .binary_search_by_key(&id, |task| task.id)
            .map_err(|_| format!("Could not find task with ID: {}", id))?;
        Ok(&self.tasks[task_index])
    }

    fn find_task_by_id_mut(&mut self, id: usize) -> Result<&mut Task, Box<dyn std::error::Error>> {
        let task_index = self
            .tasks
            .binary_search_by_key(&id, |task| task.id)
            .map_err(|_| format!("Could not find task with ID: {}", id))?;
        Ok(&mut self.tasks[task_index])
    }
}

#[derive(Default, Serialize, Deserialize, Debug)]
struct Database {
    projects: Vec<Project>,
    next_project_id: usize,
}

impl Database {
    fn find_project_by_id(&self, id: usize) -> Result<&Project, Box<dyn std::error::Error>> {
        let project_index = self
            .projects
            .binary_search_by_key(&id, |project| project.id)
            .map_err(|_| format!("Could not find project with ID: {}", id))?;
        Ok(&self.projects[project_index])
    }

    fn find_project_by_id_mut(
        &mut self,
        id: usize,
    ) -> Result<&mut Project, Box<dyn std::error::Error>> {
        let project_index = self
            .projects
            .binary_search_by_key(&id, |project| project.id)
            .map_err(|_| format!("Could not find project with ID: {}", id))?;
        Ok(&mut self.projects[project_index])
    }
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

    fn flush(&self) -> Result<(), Box<dyn std::error::Error>> {
        let database_path = Self::get_database_path();
        let dirname = database_path
            .parent()
            .expect("Expected path to be absolute");
        std::fs::create_dir_all(dirname)?;
        serde_json::to_writer(File::create(database_path)?, &self.database)?;
        Ok(())
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
struct ProjectDetailsRequest {
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
    let request = serde_json::from_slice::<ProjectDetailsRequest>(&full_body)?;
    let app = app_state.lock().unwrap();
    let project = app.database.find_project_by_id(request.project_id)?;
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
        })
        .to_string(),
    )))
}

#[derive(Deserialize, Debug)]
struct TaskDetailsRequest {
    project_id: usize,
    task_id: usize,
}

async fn task_details(
    request: Request<Body>,
    app_state: Arc<Mutex<AppState>>,
) -> Result<Response<Body>, Box<dyn std::error::Error>> {
    let full_body = hyper::body::to_bytes(request.into_body()).await?;
    let request = serde_json::from_slice::<TaskDetailsRequest>(&full_body)?;
    let app = app_state.lock().unwrap();
    let project = app.database.find_project_by_id(request.project_id)?;
    let task = project.find_task_by_id(request.task_id)?;
    Ok(Response::new(Body::from(serde_json::to_string(task)?)))
}

#[derive(Deserialize, Debug)]
struct PostTaskCommentRequest {
    project_id: usize,
    task_id: usize,
    comment: String,
}

async fn post_task_comment(
    request: Request<Body>,
    app_state: Arc<Mutex<AppState>>,
) -> Result<Response<Body>, Box<dyn std::error::Error>> {
    let full_body = hyper::body::to_bytes(request.into_body()).await?;
    let request = serde_json::from_slice::<PostTaskCommentRequest>(&full_body)?;
    let mut app = app_state.lock().unwrap();
    let project = app.database.find_project_by_id_mut(request.project_id)?;
    let task = project.find_task_by_id_mut(request.task_id)?;
    task.new_log_entry(LogEntryType::Comment(request.comment));
    app.flush()?;
    Ok(Response::new(Body::from(
        json!({"status": 200, "description": "OK"}).to_string(),
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
        (&Method::GET, "/") => wrap_error(list_projects(request, app_state).await),
        (&Method::GET, "/project") => wrap_error(project_details(request, app_state).await),
        (&Method::GET, "/task") => wrap_error(task_details(request, app_state).await),
        (&Method::POST, "/task/comment") => wrap_error(post_task_comment(request, app_state).await),
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
