# Btasks

This is a simple task management tool I've been writing over the weekend. I don't want to create accounts to use task management software online. Another plus is that the underlying database for this tool is in plain old human-readable JSON, so I can go poke around in it whenever I want.

## Architecture

The core component is the `btasks` server in Rust, for which I've used [Tokio](https://tokio.rs/) and [Hyper](https://hyper.rs/). Clients can query and also write to it via a fairly simple HTTP+JSON API.

Note that this API has (next to) no error reporting at all. For clearly wrong requests, it will happily spit out `{"status":200,"description":"OK"}`. Also there's no authentication at all. It's a task handling API, come on!

## API documentation

- [List projects](#list-projects) : `GET /`
- [Project details](#project-details) : `GET /project`
- [Create project](#create-project) : `POST /project/create`
- [Rename project](#rename-project) : `POST /project/name`
- [Set project description](#set-project-description) : `POST /project/description`
- [Task details](#task-details) : `GET /task`
- [Create task](#create-tas) : `POST /task/create`
- [Set task title](#set-task-title) : `POST /task/title`
- [Set task description](#set-task-description) : `POST /task/description`
- [Add/remove dependency](#addremove-dependency) : `POST /task/dependency`
- [Change task state](#change-task-state) : `POST /task/state`
- [Post comment on task](#post-comment-on-task) : `POST /task/comment`

### List projects

- URL : `/`
- Method : `GET`
- Body : --empty--

Success response -
```json
{
  "projects": [
    {
      "id": 0,
      "name": "Project A"
    },
    {
      "id": 1,
      "name": "Project B"
    }
  ]
}
```

### Project details

- URL : `/project`
- Method : `GET`
- Body : `{ "project_id" : 0 }`

Success response -
```json
{
  "name" : "Project A",
  "id" : 0,
  "description" : "Sample project",
  "tasks" : [
    {
      "title" : "Task A",
      "state" : "Todo",
      "id" : 0
    },
    {
      "title" : "Task B",
      "state" : "InProgress",
      "id" : 1
    }
  ]
}
```

### Create project

- URL : `/project/create`
- Method : `POST`
- Body : `{ "name" : "Project C", "description" : "Another project" }`

Success response -
```json
{
  "project_id" : 3
}
```

### Rename project

- URL : `/project/name`
- Method : `POST`
- Body : `{ "project_id" : 0, "name" : "Project Z" }`

Success response -
```json
{
  "status" : 200,
  "description" : "OK"
}
```

### Set project description

- URL : `/project/description`
- Method : `POST`
- Body : `{ "project_id" : 0, "description" : "Updated description" }`

Success response -
```json
{
  "status" : 200,
  "description" : "OK"
}
```

### Task details

- URL : `/task`
- Method : `GET`
- Body : `{ "project_id" : 0, "task_id" : 0 }`

Success response -
```json
{
  "title" : "Task A",
  "id" : 0,
  "description" : "Something I have to do",
  "state" : "Blocked",
  "log" : [
    {
      "timestamp" : 1645383320,
      "entry_type" : {
        "Comment" : "Sample comment"
      }
    },
    {
      "timestamp" : 1645383352,
      "entry_type" : {
        "StateChangedTo" : "Blocked"
      }
    }
  ],
  "dependencies" : [ 1 ]
}
```

### Create task

- URL : `/task/create`
- Method : `POST`
- Body : `{ "project_id" : 0, "name" : "Task C", "description" : "Another task" }`

Success response -
```json
{
  "task_id" : 2
}
```

### Set task title

- URL : `/task/title`
- Method : `POST`
- Body : `{ "project_id" : 0, "task_id" : 0, "title" : "Task Z" }`

Success response -
```json
{
  "status" : 200,
  "description" : "OK"
}
```

### Set task description

- URL : `/task/description`
- Method : `POST`
- Body : `{ "project_id" : 0, "task_id" : 0, "description" : "Updated task description" }`

Success response -
```json
{
  "status" : 200,
  "description" : "OK"
}
```

### Add/remove dependency

- URL : `/task/dependency`
- Method : `POST`
- Body : `{ "project_id" : 0, "task_id" : 0, "action" : "Add", "dependency" : 2 }`

Success response -
```json
{
  "status" : 200,
  "description" : "OK"
}
```

### Change task state

- URL : `/task/state`
- Method : `POST`
- Body : `{ "project_id" : 0, "task_id" : 0, "new_state" : "Done" }`

Success response -
```json
{
  "status" : 200,
  "description" : "OK"
}
```

### Post comment on task

- URL : `/task/comment`
- Method : `POST`
- Body : `{ "project_id" : 0, "task_id" : 0, "comment" : "Blah" }`

Success response -
```json
{
  "status" : 200,
  "description" : "OK"
}
```
