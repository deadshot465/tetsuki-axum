use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};

pub async fn initialize_db() -> anyhow::Result<Pool<Postgres>> {
    let connection_string = dotenv::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new().connect(&connection_string).await?;
    Ok(pool)
}
