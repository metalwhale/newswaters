use std::{env, io::Write};

use anyhow::{bail, Context, Error, Result};
use chrono::{DateTime, Local};
use diesel::{
    deserialize::{FromSql, FromSqlRow},
    expression::AsExpression,
    pg::Pg,
    prelude::*,
    serialize::{IsNull, ToSql},
    sql_types::*,
};

use crate::{
    schema::{item_urls, items, sql_types::ItemType},
    service::{Item, ItemUrl},
};

pub(crate) struct ItemRepository {
    connection: PgConnection,
}

impl ItemRepository {
    pub(crate) fn new() -> Result<Self> {
        let database_url = format!(
            "postgres://{}:{}@{}/{}",
            env::var("DATABASE_USER")?,
            env::var("DATABASE_PASSWORD")?,
            env::var("DATABASE_HOST")?,
            env::var("DATABASE_DB")?,
        );
        return Ok(Self {
            connection: PgConnection::establish(&database_url)?,
        });
    }

    pub(crate) fn find_min_item_id(&mut self) -> Result<i32> {
        let min_item_id = diesel::sql_query("SELECT id FROM items ORDER BY id ASC LIMIT 1")
            .get_result::<ItemIdRecord>(&mut self.connection)?
            .id;
        return Ok(min_item_id);
    }

    pub(crate) fn find_max_item_id(&mut self) -> Result<i32> {
        let max_item_id = diesel::sql_query("SELECT id FROM items ORDER BY id DESC LIMIT 1")
            .get_result::<ItemIdRecord>(&mut self.connection)?
            .id;
        return Ok(max_item_id);
    }

    pub(crate) fn find_missing_items(&mut self, min_id: i32, max_id: i32) -> Result<Vec<i32>> {
        // See: https://stackoverflow.com/questions/12444142/postgresql-how-to-figure-out-missing-numbers-in-a-column-using-generate-series
        let missing_items = diesel::sql_query(
            "SELECT s.i AS id \
            FROM generate_series($1, $2) AS s(i) \
            WHERE NOT EXISTS (SELECT 1 FROM items WHERE id = s.i) \
            ORDER BY id ASC",
        )
        .bind::<Integer, _>(min_id)
        .bind::<Integer, _>(max_id)
        .get_results::<MissingItemRecord>(&mut self.connection)?
        .iter()
        .map(|r| r.id)
        .collect();
        return Ok(missing_items);
    }

    pub(crate) fn find_missing_item_urls(&mut self, min_id: i32, max_id: i32) -> Result<Vec<(i32, String)>> {
        let missing_item_urls = diesel::sql_query(
            "SELECT s.i AS id, s.u AS url \
            FROM (SELECT id, url FROM items WHERE id >= $1 AND id <= $2 AND url IS NOT NULL ORDER BY id ASC) AS s(i, u) \
            WHERE NOT EXISTS (SELECT 1 FROM item_urls WHERE item_id = s.i) \
            ORDER BY id ASC",
        )
        .bind::<Integer, _>(min_id)
        .bind::<Integer, _>(max_id)
        .get_results::<MissingItemUrlRecord>(&mut self.connection)?
        .into_iter()
        .map(|r| (r.id, r.url))
        .collect();
        return Ok(missing_item_urls);
    }

    pub(crate) fn save_item(&mut self, item: Item) -> Result<()> {
        let item_record = ItemRecord {
            id: item.id,
            deleted: item.deleted,
            type_: Some(item.type_.context(format!("item.id={}", item.id))?.try_into()?),
            by: item.by,
            time: item.time,
            text: item.text,
            dead: item.dead,
            parent: item.parent,
            poll: item.poll,
            url: item.url,
            score: item.score,
            title: item.title,
            descendants: item.descendants,
            created_at: Local::now(),
            updated_at: Local::now(),
        };
        diesel::insert_into(items::table)
            .values(&item_record)
            .returning(ItemRecord::as_returning())
            .get_result(&mut self.connection)?;
        Ok(())
    }

    pub(crate) fn save_item_url(&mut self, item_id: i32, item_url: ItemUrl) -> Result<()> {
        let mut item_url_record = ItemUrlRecord {
            item_id,
            html: None,
            text: None,
            summary: None,
            status_code: None,
            status_note: None,
            created_at: Local::now(),
            updated_at: Local::now(),
        };
        // TODO: Use enum to encapsulate `status_code` value
        match item_url {
            ItemUrl::Finished { html, text } => {
                item_url_record.html = Some(html);
                item_url_record.text = Some(text);
                item_url_record.status_code = Some(0);
            }
            ItemUrl::Skipped { note } => {
                item_url_record.status_note = Some(note);
                item_url_record.status_code = Some(1);
            }
            ItemUrl::Canceled { note } => {
                item_url_record.status_note = Some(note);
                item_url_record.status_code = Some(2);
            }
        }
        diesel::insert_into(item_urls::table)
            .values(&item_url_record)
            .returning(ItemUrlRecord::as_returning())
            .get_result(&mut self.connection)?;
        Ok(())
    }
}

#[derive(AsExpression, FromSqlRow, Debug)]
#[diesel(sql_type = ItemType)]
enum ItemTypeValue {
    Job,
    Story,
    Comment,
    Poll,
    Pollopt,
}

impl TryFrom<String> for ItemTypeValue {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "job" => Ok(ItemTypeValue::Job),
            "story" => Ok(ItemTypeValue::Story),
            "comment" => Ok(ItemTypeValue::Comment),
            "poll" => Ok(ItemTypeValue::Poll),
            "pollopt" => Ok(ItemTypeValue::Pollopt),
            _ => bail!(""),
        }
    }
}

// See: https://github.com/diesel-rs/diesel/blob/master/diesel_tests/tests/custom_types.rs
impl ToSql<ItemType, Pg> for ItemTypeValue {
    fn to_sql<'b>(&'b self, out: &mut diesel::serialize::Output<'b, '_, Pg>) -> diesel::serialize::Result {
        match *self {
            ItemTypeValue::Job => out.write_all(b"job")?,
            ItemTypeValue::Story => out.write_all(b"story")?,
            ItemTypeValue::Comment => out.write_all(b"comment")?,
            ItemTypeValue::Poll => out.write_all(b"poll")?,
            ItemTypeValue::Pollopt => out.write_all(b"pollopt")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<ItemType, Pg> for ItemTypeValue {
    fn from_sql(bytes: <Pg as diesel::backend::Backend>::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        Ok(<String as FromSql<Text, Pg>>::from_sql(bytes)?.try_into()?)
    }
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = items)]
#[diesel(check_for_backend(Pg))]
struct ItemRecord {
    id: i32,
    deleted: Option<bool>,
    type_: Option<ItemTypeValue>,
    by: Option<String>,
    time: Option<i64>,
    text: Option<String>,
    dead: Option<bool>,
    parent: Option<i32>,
    poll: Option<i32>,
    url: Option<String>,
    score: Option<i32>,
    title: Option<String>,
    descendants: Option<i32>,
    created_at: DateTime<Local>,
    updated_at: DateTime<Local>,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = item_urls)]
#[diesel(check_for_backend(Pg))]
struct ItemUrlRecord {
    item_id: i32,
    html: Option<String>,
    text: Option<String>,
    summary: Option<String>,
    status_code: Option<i32>,
    status_note: Option<String>,
    created_at: DateTime<Local>,
    updated_at: DateTime<Local>,
}

#[derive(QueryableByName)]
struct ItemIdRecord {
    #[diesel(sql_type = Integer)]
    id: i32,
}

#[derive(QueryableByName)]
struct MissingItemRecord {
    #[diesel(sql_type = Integer)]
    id: i32,
}

#[derive(QueryableByName)]
struct MissingItemUrlRecord {
    #[diesel(sql_type = Integer)]
    id: i32,
    #[diesel(sql_type = Text)]
    url: String,
}