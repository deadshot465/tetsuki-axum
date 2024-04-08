#![allow(dead_code)]
use crate::shared::configuration::CONFIGURATION;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};

pub fn initialize_db() -> anyhow::Result<Pool<Postgres>> {
    let connection_string = &CONFIGURATION.database_url;
    let pool = PgPoolOptions::new().connect_lazy(connection_string)?;
    Ok(pool)
}
