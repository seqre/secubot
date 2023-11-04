use std::sync::Arc;

use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use itertools::Itertools;
use poise::serenity_prelude::{async_trait, futures, CacheHttp, ChannelId, Http};
use tokio::time::Duration;

use crate::{ctx_data::CtxData, models::todo::Todo, settings::Feature, tasks::Task, Conn};

pub struct TodoReminderTask {
    ctx_data: Arc<CtxData>,
    http: Arc<dyn CacheHttp>,
}

impl TodoReminderTask {
    pub fn new(ctx_data: Arc<CtxData>, http: Arc<Http>) -> Self {
        Self { ctx_data, http }
    }
}

#[async_trait]
impl Task for TodoReminderTask {
    fn get_interval(&self) -> Duration {
        // Every 5 days
        Duration::from_secs(60 * 60 * 24 * 5)
    }

    #[allow(clippy::cast_sign_loss)]
    async fn work(&self) {
        use crate::schema::todos::dsl::{completion_date, todos};

        let results = todos
            .filter(completion_date.is_null())
            .load::<Todo>(&mut self.ctx_data.db.get().unwrap());

        let channels: Vec<(ChannelId, usize)> = results
            .unwrap()
            .into_iter()
            .group_by(|td| td.channel_id)
            .into_iter()
            .map(|(chnl, tds)| (ChannelId(chnl as u64), tds.count()))
            .collect();

        for (chnl, count) in channels {
            _ = chnl
                .send_message(&self.http.http(), |message| {
                    message.embed(|embed| {
                        embed.title("TODOs reminder");
                        embed.description(format!("There are {count} uncompleted TODOs here!"));
                        embed
                    })
                })
                .await;
        }
    }
}
