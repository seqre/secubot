use std::sync::{Arc, Mutex};

use diesel::sqlite::SqliteConnection;

pub type Conn = Arc<Mutex<SqliteConnection>>;

pub struct Secubot {
    pub db: Conn,
}

impl Secubot {
    pub fn new(db: Conn) -> Self {
        Self { db }
    }
}
