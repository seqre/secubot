use diesel::{Insertable, Queryable};

use crate::schema::todos;

#[derive(Queryable, Debug)]
pub struct Todo {
    pub id: i32,
    pub channel_id: i64,
    pub todo: String,
    pub creation_date: String,
    pub completion_date: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = todos)]
pub struct NewTodo<'a> {
    pub channel_id: &'a i64,
    pub id: &'a i32,
    pub todo: &'a str,
    pub creation_date: &'a str,
}
