use actix_web::{App, HttpResponse, HttpServer, get, post, web};
use askama::Template;
use askama_actix::TemplateToResponse;
use sqlx::{FromRow, SqlitePool};

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

#[derive(FromRow)]
struct TodoItem {
    id: i64,
    task: String,
    priority: u32,
}

#[get("/")]
async fn todo(pool: web::Data<SqlitePool>) -> HttpResponse {
    // SQLクエリで直接TodoItemにマッピング
    let items = sqlx::query_as::<_, TodoItem>("SELECT id, task, COALESCE(priority, 0) as priority FROM tasks ORDER BY priority ASC, id ASC;")
        .fetch_all(pool.as_ref())
        .await
        .unwrap();

    let todo = TodoTemplate { items };
    todo.to_response()
}

#[derive(serde::Deserialize)]
struct Task {
    id: Option<i64>,
    task: Option<String>,
    priority: Option<u32>,
}

#[post("/update")]
async fn update(pool: web::Data<SqlitePool>, form: web::Form<Task>) -> HttpResponse {
    let received_task = form.into_inner(); // シャドウイングを避けるため、変数をリネーム

    // 削除処理
    if let Some(id) = received_task.id {
        sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(id)
            .execute(pool.as_ref())
            .await
            .unwrap();
    }

    // 挿入/更新処理
    if let Some(task_content) = received_task.task { // String型の中身を取り出す
        if !task_content.is_empty() { // 文字列が空でないことを確認
            // priorityは元のreceived_taskから取得
            let priority = received_task.priority.unwrap_or(0); // Noneの場合にデフォルト値0を設定

            sqlx::query("INSERT INTO tasks (task,priority) VALUES(?,?)")
                .bind(task_content) // task_content (String) をバインド
                .bind(priority)
                .execute(pool.as_ref())
                .await
                .unwrap();
        }
    }
    HttpResponse::Found()
        .append_header(("Location", "/"))
        .finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
        "
        CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task TEXT NOT NULL,
            priority INTEGER
        )
    ",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query("INSERT INTO tasks (task,priority) VALUES ('task1',1)")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO tasks (task,priority) VALUES ('task2',2)")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO tasks (task,priority) VALUES ('task3',3)")
        .execute(&pool)
        .await
        .unwrap();

    HttpServer::new(move || {
        App::new()
            .service(hello)
            .service(update)
            .service(todo)
            .app_data(web::Data::new(pool.clone()))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
