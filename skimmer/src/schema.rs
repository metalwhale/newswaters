// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "item_type"))]
    pub struct ItemType;
}

diesel::table! {
    analyses (item_id) {
        item_id -> Int4,
        keyword -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    item_urls (item_id) {
        item_id -> Int4,
        html -> Nullable<Text>,
        text -> Nullable<Text>,
        summary -> Nullable<Text>,
        status_code -> Nullable<Int4>,
        status_note -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::ItemType;

    items (id) {
        id -> Int4,
        deleted -> Nullable<Bool>,
        #[sql_name = "type"]
        type_ -> Nullable<ItemType>,
        by -> Nullable<Text>,
        time -> Nullable<Int8>,
        text -> Nullable<Text>,
        dead -> Nullable<Bool>,
        parent -> Nullable<Int4>,
        poll -> Nullable<Int4>,
        url -> Nullable<Text>,
        score -> Nullable<Int4>,
        title -> Nullable<Text>,
        descendants -> Nullable<Int4>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(analyses -> items (item_id));
diesel::joinable!(item_urls -> items (item_id));

diesel::allow_tables_to_appear_in_same_query!(
    analyses,
    item_urls,
    items,
);
