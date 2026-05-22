use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::path::PathBuf;

pub async fn init_pool(app_data_dir: PathBuf) -> Result<SqlitePool, sqlx::Error> {
    let db_dir = app_data_dir.join(".spytfy");
    std::fs::create_dir_all(&db_dir).expect("failed to create .spytfy directory");

    let db_path = db_dir.join("db.sqlite");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
