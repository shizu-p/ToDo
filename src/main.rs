use actix_web::{App, HttpResponse, HttpServer, get, post, web};
use askama::Template;
use askama_actix::TemplateToResponse;
use sqlx::{Row, SqlitePool};
use sqlx::FromRow;
use actix_files as fs;


#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate {
    name: String,
}

#[get("/hello/{name}")]
async fn hello(name: web::Path<String>) -> HttpResponse {
    let hello = HelloTemplate {
        name: name.into_inner(),
    };
    hello.to_response()
}

#[derive(Template)]
#[template(path = "todo.html")]
struct TodoTemplate {
    items: Vec<TodoItem>,
}

#[derive(FromRow,Debug,Clone)]
struct TodoItem{
    id:i64,
    task:String,
    priority:Option<u32>, // NULL を許容するため
}


#[get("/")]
async fn todo(pool: web::Data<SqlitePool>) -> HttpResponse {
    // SQLクエリで直接TodoItemにマッピング
    let items = sqlx::query_as::<_, TodoItem>("SELECT id,task,priority FROM tasks ORDER BY priority ASC,id ASC;")
        .fetch_all(pool.as_ref())
        .await
        .unwrap();

    let todo = TodoTemplate { items };
    todo.to_response()
}

#[derive(serde::Deserialize)]
struct AddTaskForm {
    task: String,
    priority:Option<u32>,
}


#[derive(serde::Deserialize)]
struct Task {
    id: Option<String>,
    task: Option<String>,
    priority: Option<u32>,
}

#[post("/add_task")]
async fn add_task(pool: web::Data<SqlitePool>,form: web::Form<addTaskForm>) -> HttpResponse {
    let new_task = form.into_inner();

    sqlx::query("INSERT INTO tasks (task,priority) VALUES(? , ?")
        .bind(&new_task.task)
        .bind(new_task,priority)
        .execute(pool.as_ref())
        .await
        .unwrap();

    HttpResponse::Found()
        .append_header(("Location","/"))
        .finish()
}

#[delete("/api/tasks/{id}")]
async fn delete_task(pool: web::Data<SqlitePool>, path_id: web::Path<i64>) -> HttpResponse {
    let task_id = path_id.into_inner();

    match sqlx::query("DELETE FROM tasks WHERE id = ?")
        .bind(task_id)
        .execute(pool.as_ref())
        .await
    {
        Ok(result) => {
            if result.rows_affected() > 0 {
                HttpResponse::Ok().json(format!("Task with id {} deleted successfully",task_id))
            } else {
                HttpResponse::NotFound().json(format!("Task with id {} not found",task_id))
            }
        }
        Err(e) => {
            // DB error
            eprintln!("Database error deleting task: {:?}", e);
            HttpResponse::InternalServerError().json("Failed to delete task")

        }
    }
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task TEXT NOT NULL,
            priority INTEGER
        )"
    )
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO tasks(task,priority) VALUES ('メールの返信',1)")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO tasks(task,priority) VALUES ('メールの返信',4)")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO tasks(task,priority) VALUES ('メールの返信',3)")
        .execute(&pool)
        .await
        .unwrap();

    HttpServer::new(move || {
        App::new()
            .service(hello)
            .service(add_task)
            .service(delete_task)
            .service(todo)
            .serivce(fs::Files::new("/static","./static"))
            .app_data(web::Data::new(pool.clone()))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
