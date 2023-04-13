use diesel::{Insertable, Queryable};

use crate::schema::todos;

#[derive(Queryable, Debug)]
pub struct Todo {
    pub channel_id: i64,
    pub id: i32,
    pub todo: String,
    pub creation_date: String,
    pub completion_date: Option<String>,
    pub assignee: Option<i64>,
    pub priority: i32,
}

#[derive(Insertable)]
#[diesel(table_name = todos)]
pub struct NewTodo<'a> {
    pub channel_id: &'a i64,
    pub id: &'a i32,
    pub todo: &'a str,
    pub creation_date: &'a str,
    pub assignee: Option<i64>,
    pub priority: i32,
}
