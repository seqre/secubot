use crate::{
    commands::{ping::PingData, todo::TodoData},
    settings::Settings,
    Conn,
};

#[derive(Debug)]
pub struct CtxData {
    pub db: Conn,
    pub ping_data: PingData,
    pub todo_data: TodoData,
    pub settings: Settings,
}

impl CtxData {
    pub fn new(db: Conn, settings: Settings) -> Self {
        let todo_data = TodoData::new(&db);
        Self {
            db,
            ping_data: PingData::new(),
            todo_data,
            settings,
        }
    }
}
