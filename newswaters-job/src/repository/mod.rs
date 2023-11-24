use diesel::PgConnection;

pub(crate) mod analysis;
pub(crate) mod item; // Core

pub(crate) struct Repository {
    connection: PgConnection,
}
