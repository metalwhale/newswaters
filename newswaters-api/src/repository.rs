use std::collections::HashMap;
use std::env;

use anyhow::Result;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use diesel::{prelude::*, sql_types::*};

pub(crate) struct Repository {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl Repository {
    pub(crate) fn new() -> Result<Self> {
        let database_url = format!(
            "postgres://{}:{}@{}:{}/{}",
            env::var("DATABASE_USER")?,
            env::var("DATABASE_PASSWORD")?,
            env::var("DATABASE_HOST")?,
            env::var("DATABASE_PORT").unwrap_or("5432".to_string()),
            env::var("DATABASE_DB")?,
        );
        let pool = Pool::builder()
            .test_on_check_out(true)
            .build(ConnectionManager::<PgConnection>::new(database_url))?;
        return Ok(Self { pool });
    }

    pub(crate) fn find_items(
        &self,
        ids: &[i32],
    ) -> Result<HashMap<i32, (Option<String>, Option<String>, Option<i64>)>> {
        let items_map = diesel::sql_query(format!(
            "SELECT id, title, url, time \
            FROM unnest(ARRAY[{}]) AS s(i) \
            JOIN items ON s.i = items.id",
            ids.iter().map(|i| i.to_string()).collect::<Vec<String>>().join(", ")
        ))
        .get_results::<ItemRecord>(&mut self.pool.get()?)?
        .into_iter()
        .map(|r| (r.id, (r.title, r.url, r.time)))
        .collect();
        Ok(items_map)
    }
}

#[derive(QueryableByName)]
struct ItemRecord {
    #[diesel(sql_type = Integer)]
    id: i32,
    #[diesel(sql_type = Nullable<Text>)]
    title: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    url: Option<String>,
    #[diesel(sql_type = Nullable<Int8>)]
    time: Option<i64>,
}
