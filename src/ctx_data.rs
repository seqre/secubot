use crate::{
    commands::{hall_of_fame::HofData, ping::PingData, todo::TodoData},
    settings::Settings,
    Conn,
};

#[derive(Debug)]
pub struct CtxData {
    pub db: Conn,
    pub ping_data: PingData,
    pub todo_data: TodoData,
    pub hof_data: HofData,
    pub settings: Settings,
}

impl CtxData {
    pub fn new(db: Conn, settings: Settings) -> Self {
        let todo_data = TodoData::new(&db);
        let hof_data = HofData::new(&db);
        Self {
            db,
            ping_data: PingData::new(),
            todo_data,
            hof_data,
            settings,
        }
    }
}
