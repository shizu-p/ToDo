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
async fn update(pool: web::Data<SqlitePool>, form: web::Form<Task>)
-> std::io::Result<HttpResponse> {
    let received_task = form.into_inner(); // シャドウイングを避けるため、変数をリネーム

    // 削除処理
    match received_task.id {
        Some(id) => {
            sqlx::query("DELETE FROM tasks WHERE id = ?")
                .bind(id)
                .execute(pool.as_ref())
                .await
                .map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("削除クエリが失敗しました : {}",e),
                    )
                });
        }
        _ => {}
    }

    // 挿入/更新処理

    match received_task.task {
        Some(task) => {
            if !task.is_empty() {
                let priority = received_task.priority.unwrap_or(0);
                sqlx::query("INSERT INTO tasks (task,priority) VALUES(?,?)")
                    .bind(task)
                    .bind(priority)
                    .execute(pool.as_ref())
                    .await
                    .map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("追加クエリが失敗しました: {}",e),
                        )
                    });
            }
        }
        _ => {}
    }

    Ok(HttpResponse::Found()
        .append_header(("Location", "/"))
        .finish())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let pool = SqlitePool::connect("sqlite::memory:").await.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("DBへの接続に失敗しました: {}", e),
        )
    })?;

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
    .map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("テーブルの作成に失敗しました:{}", e),
        )
    });

    sqlx::query("INSERT INTO tasks (task,priority) VALUES ('task1',1)")
        .execute(&pool)
        .await
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("タスクの作成に失敗しました: {}", e),
            )
        });

    sqlx::query("INSERT INTO tasks (task,priority) VALUES ('task2',2)")
        .execute(&pool)
        .await
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("タスクの作成に失敗しました: {}", e),
            )
        });

    sqlx::query("INSERT INTO tasks (task,priority) VALUES ('task3',3)")
        .execute(&pool)
        .await
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("タスクの作成に失敗しました: {}", e),
            )
        });

    HttpServer::new(move || {
        App::new()
            .service(hello)
            .service(update)
            .service(todo)
            .app_data(web::Data::new(pool.clone()))
    })
    .bind(("0.0.0.0", 10000))?
    .run()
    .await?;

    Ok(())
}
