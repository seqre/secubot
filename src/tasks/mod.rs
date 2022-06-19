use std::sync::Arc;

use serenity::{async_trait, http::client::Http};
use tokio::{
    task,
    time::{sleep, Duration},
};

use crate::{secubot::Secubot, tasks::todo_reminder::TodoReminderTask};

mod todo_reminder;

#[async_trait]
pub trait Task: Send + Sync {
    fn get_interval(&self) -> Duration;
    async fn work(&self);
}

pub struct Tasks;

impl Tasks {
    // FIXME: the whole implementation works, but it less than ideal
    pub fn new() -> Self {
        Self {}
    }

    fn get_tasks(secubot: &Secubot, http: Arc<Http>) -> Vec<Box<dyn Task>> {
        let tasks: Vec<Box<dyn Task>> = vec![Box::new(TodoReminderTask::new(secubot, http))];
        tasks
    }

    pub fn start_tasks(&self, secubot: &Secubot, http: Arc<Http>) {
        for task in Tasks::get_tasks(secubot, http) {
            task::spawn(async move {
                loop {
                    task.work().await;
                    sleep(task.get_interval()).await;
                }
            });
        }
    }
}
