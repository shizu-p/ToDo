use actix_files::Files;
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
async fn todo(pool: web::Data<SqlitePool>) -> std::io::Result<HttpResponse> {
    // SQLクエリで直接TodoItemにマッピング
    let items = sqlx::query_as::<_, TodoItem>("SELECT id, task, COALESCE(priority, 0) as priority FROM tasks ORDER BY priority ASC, id ASC;")
        .fetch_all(pool.as_ref())
        .await
        .map_err(|e|{
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("タスクの取得に失敗しました: {}",e),
            )
        })?;

    let todo = TodoTemplate { items };
    Ok(todo.to_response())
}

#[derive(serde::Deserialize)]
struct Task {
    id: Option<i64>,
    task: Option<String>,
    priority: Option<u32>,
}

// タスク追加クエリ用 構造体 impl
#[derive(serde::Deserialize)]
struct NewTask {
    task: String,
    priority: u32,
}

impl NewTask {
    async fn insert_newTask(&self, pool: &SqlitePool) -> std::io::Result<()> {
        if self.task.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "タスク内容は空に出来ません",
            ));
        }

        sqlx::query("INSERT INTO tasks (task,priority) VALUES(?,?)")
            .bind(&self.task)
            .bind(self.priority)
            .execute(pool)
            .await
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("追加クエリが失敗しました: {}", e),
                )
            })?;
        Ok(())
    }
}

struct DeleteTask {
    id: i64,
}

impl DeleteTask {
    async fn delete_by_id(&self, pool: &SqlitePool) -> std::io::Result<()> {
        sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(&self.id)
            .execute(pool)
            .await
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("削除クエリが失敗しました: {}", e),
                )
            })?;
        Ok(())
    }
}

#[post("/update")]
async fn update(
    pool: web::Data<SqlitePool>,
    form: web::Form<Task>,
) -> std::io::Result<HttpResponse> {
    let received_task = form.into_inner(); // シャドウイングを避けるため、変数をリネーム

    // 削除処理
    match received_task.id {
        Some(id) => {
            DeleteTask { id: id }.delete_by_id(&pool).await?;
        }
        _ => {}
    }

    // 挿入/更新処理

    match received_task.task {
        Some(task) => {
            if !task.is_empty() {
                let priority = received_task.priority.unwrap_or(0);
                NewTask { task, priority }.insert_newTask(&pool).await?;
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
    })?;

    // 初期レコード
    NewTask {
        task: "タスク1".to_string(),
        priority: 1,
    }
    .insert_newTask(&pool)
    .await?;
    NewTask {
        task: "タスク2".to_string(),
        priority: 2,
    }
    .insert_newTask(&pool)
    .await?;
    NewTask {
        task: "タスク3".to_string(),
        priority: 3,
    }
    .insert_newTask(&pool)
    .await?;

    HttpServer::new(move || {
        App::new()
            .service(hello)
            .service(update)
            .service(todo)
            .app_data(web::Data::new(pool.clone()))
            .service(Files::new("/css", "./static/css").show_files_listing())
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await?;

    Ok(())
}
