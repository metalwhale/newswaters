use std::collections::HashMap;
use std::env;

use anyhow::Result;
use diesel::{prelude::*, sql_types::*};
use diesel::{Connection, PgConnection};

pub(crate) struct Repository {
    connection: PgConnection,
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
        return Ok(Self {
            connection: PgConnection::establish(&database_url)?,
        });
    }

    pub(crate) fn find_items(&mut self, ids: &[i32]) -> Result<HashMap<i32, (String, String)>> {
        let items_map = diesel::sql_query(format!(
            "SELECT id, title, url \
            FROM unnest(ARRAY[{}]) AS s(i) \
            JOIN items ON s.i = items.id",
            ids.iter().map(|i| i.to_string()).collect::<Vec<String>>().join(", ")
        ))
        .get_results::<ItemRecord>(&mut self.connection)?
        .into_iter()
        .map(|r| (r.id, (r.title, r.url)))
        .collect();
        Ok(items_map)
    }
}

#[derive(QueryableByName)]
struct ItemRecord {
    #[diesel(sql_type = Integer)]
    id: i32,
    #[diesel(sql_type = Text)]
    title: String,
    #[diesel(sql_type = Text)]
    url: String,
}
