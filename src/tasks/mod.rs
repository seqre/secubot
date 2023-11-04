use std::sync::Arc;

use poise::serenity_prelude::{async_trait, Http};
use tokio::time::{sleep, Duration};

use crate::{ctx_data::CtxData, settings::Feature, tasks::todo_reminder::TodoReminderTask};

mod todo_reminder;

#[async_trait]
pub trait Task: Send + Sync {
    fn get_interval(&self) -> Duration;
    async fn work(&self);
}

// FIXME: the whole implementation works, but it less than ideal

fn get_tasks(ctx_data: &Arc<CtxData>, http: Arc<Http>) -> Vec<Box<dyn Task>> {
    let tasks: Vec<Box<dyn Task>> = vec![Box::new(TodoReminderTask::new(ctx_data.clone(), http))];
    tasks
}

pub fn start_tasks(ctx_data: &Arc<CtxData>, http: Arc<Http>) {
    for task in get_tasks(ctx_data, http) {
        tokio::task::spawn(async move {
            loop {
                task.work().await;
                sleep(task.get_interval()).await;
            }
        });
    }
}
