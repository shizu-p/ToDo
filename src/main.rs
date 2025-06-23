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
struct TaskPayload {
    action: String,
    id: Option<i64>,
    task: Option<String>,
    priority: Option<u32>,
}

impl TaskPayload {
    async fn execute_action(&self, pool: &SqlitePool) -> std::io::Result<()> {
        match self.action.as_str() {
            "delete" => {
                let id = self.id.ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "削除にはタスクIDが必要です",
                    )
                })?;
                sqlx::query("DELETE FROM tasks WHERE id = ?")
                    .bind(id)
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
            "add" => {
                let task = self.task.as_ref().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "追加にはタスク内容が必要です",
                    )
                })?;

                let priority = self.priority.ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "追加には優先度が必要です",
                    )
                })?;

                if task.is_empty() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "タスク内容は空にできません",
                    ));
                }
                sqlx::query("INSERT INTO tasks (task,priority) VALUES(?,?)")
                    .bind(&task)
                    .bind(priority)
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
            "edit" => {
                let id = self.id.ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "削除にはタスクIDが必要です",
                    )
                })?;
                let task = self.task.as_ref().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "追加にはタスク内容が必要です",
                    )
                })?;
                let priority = self.priority.ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "追加には優先度が必要です",
                    )
                })?;
                sqlx::query("UPDATE tasks SET task=? ,priority=? WHERE id = ?")
                    .bind(task)
                    .bind(priority)
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("編集クエリが失敗しました: {}", e),
                        )
                    })?;
                Ok(())
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("不明なアクション :{}", self.action),
            )),
        }
    }
}

#[post("/update")]
async fn update(
    pool: web::Data<SqlitePool>,
    form: web::Form<TaskPayload>,
) -> std::io::Result<HttpResponse> {
    let received_payload = form.into_inner(); // シャドウイングを避けるため、変数をリネーム

    received_payload.execute_action(&pool).await?;

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
    let initial_tasks = vec![("タスク1", 1u32), ("タスク2", 2u32), ("タスク3", 3u32)];
    for (task_name, priority) in initial_tasks {
        let payload = TaskPayload {
            action: "add".to_string(),
            id: None,
            task: Some(task_name.to_string()),
            priority: Some(priority),
        };
        payload.execute_action(&pool).await.map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("初期データの挿入に失敗しました: {}", e),
            )
        })?;
    }
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
