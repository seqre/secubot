use diesel::{
    r2d2::{ConnectionManager, Pool},
    sqlite::SqliteConnection,
};

pub type Conn = Pool<ConnectionManager<SqliteConnection>>;

pub struct Secubot {
    pub db: Conn,
}

impl Secubot {
    pub fn new(db: Conn) -> Self {
        Self { db }
    }
}
