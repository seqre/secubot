use diesel::sqlite::SqliteConnection;
use std::{
    sync::{Arc, Mutex},
};

pub type Conn = Arc<Mutex<SqliteConnection>>;

pub struct Secubot {
    pub db: Conn,
}

impl Secubot {
    pub fn new(db: Conn) -> Self {
        Self { db }
    }
}
