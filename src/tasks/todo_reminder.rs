use crate::models::*;
use crate::secubot::{Conn, Secubot};
use crate::tasks::Task;
use async_trait::async_trait;
use serenity::http::client::Http;
use serenity::model::id::ChannelId;
use std::sync::Arc;
use tokio::time::Duration;

pub struct TodoReminderTask {
    db: Conn,
    http: Arc<Http>,
}

impl TodoReminderTask {
    pub fn new(secubot: &Secubot, http: Arc<Http>) -> Self {
        Self {
            db: secubot.db.clone(),
            http,
        }
    }
}

#[async_trait]
impl Task for TodoReminderTask {
    fn get_interval(&self) -> Duration {
        // Every 5 days
        Duration::from_secs(60 * 60 * 24 * 5)
    }

    async fn work(&self) {
        use crate::schema::todos::dsl::*;
        use crate::*;
        use itertools::Itertools;

        let results = todos
            .filter(completion_date.is_null())
            .load::<Todo>(&*self.db.lock().unwrap());

        let channels: Vec<(ChannelId, u32)> = results
            .unwrap()
            .into_iter()
            .group_by(|td| td.channel_id)
            .into_iter()
            .map(|(chnl, tds)| (ChannelId(chnl as u64), tds.count() as u32))
            .collect();

        for (chnl, count) in channels {
            chnl.send_message(&self.http, |message| {
                message.embed(|embed| {
                    embed.description(format!("There are {} uncompleted TODOs here!", count));
                    embed.title("TODOs reminder");
                    embed
                })
            })
            .await;
        }
    }
}
