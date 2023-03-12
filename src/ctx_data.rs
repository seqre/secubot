use crate::{
    commands::{ping::PingData, todo::TodoData},
    Conn,
};

#[derive(Debug)]
pub struct CtxData {
    pub db: Conn,
    pub ping_data: PingData,
    pub todo_data: TodoData,
}

impl CtxData {
    pub fn new(db: Conn) -> Self {
        let todo_data = TodoData::new(&db);
        Self {
            db,
            ping_data: PingData::new(),
            todo_data,
        }
    }
}
