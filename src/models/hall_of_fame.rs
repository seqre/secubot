use diesel::{Insertable, Queryable};

use crate::schema::{hall_of_fame_entries, hall_of_fame_tables};

#[derive(Queryable, Debug)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Table {
    pub id: i32,
    pub guild_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub creation_date: String,
}

#[derive(Queryable, Debug)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Entry {
    pub id: i32,
    pub hof_id: i32,
    pub user_id: i64,
    pub description: Option<String>,
    pub creation_date: String,
}

#[derive(Insertable)]
#[diesel(table_name = hall_of_fame_tables)]
pub struct NewTable<'a> {
    pub guild_id: &'a i64,
    pub title: &'a str,
    pub description: Option<String>,
    pub creation_date: &'a str,
}

#[derive(Insertable)]
#[diesel(table_name = hall_of_fame_entries)]
pub struct NewEntry<'a> {
    pub hof_id: &'a i32,
    pub user_id: &'a i64,
    pub description: Option<&'a str>,
    pub creation_date: &'a str,
}
