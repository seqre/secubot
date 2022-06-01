use async_trait::async_trait;
use tokio::{
    task,
    time::{sleep, Duration},
};


#[async_trait]
pub trait Task: Send + Sync {
    fn get_interval(&self) -> Duration;
    async fn work(&self);
}

pub struct Tasks;

impl Tasks {
    //FIXME: the whole implementation works, but it less than ideal
    pub fn new() -> Self {
        Self {}
    }

    fn get_tasks() -> Vec<Box<dyn Task>> {
        let tasks: Vec<Box<dyn Task>> = vec![];
        tasks
    }

    pub fn start_tasks(&self) {
        for task in Tasks::get_tasks() {
            task::spawn(async move {
                loop {
                    task.work().await;
                    sleep(task.get_interval()).await;
                }
            });
        }
    }
}
