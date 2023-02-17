use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use lazy_static::lazy_static;
use log::debug;
use poise::serenity_prelude::{ChannelId, Http, UserId};
use regex::Regex;
use tokio::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
    task::JoinHandle,
    time::sleep,
};

use crate::{Context, Result};

const PING_CHANNEL_BUFFER: usize = 15;
const PING_TIMEOUT: Duration = Duration::from_secs(60 * 10);

#[derive(Debug)]
pub struct PingData {
    _worker: JoinHandle<()>,
    channel: Sender<PingWorkerMessage>,
}

impl PingData {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<PingWorkerMessage>(PING_CHANNEL_BUFFER);
        let mut worker = PingWorker::new(rx);
        let handle = tokio::spawn(async move {
            worker.work().await;
        });
        Self {
            _worker: handle,
            channel: tx,
        }
    }
}

#[poise::command(slash_command, subcommands("commence", "remove", "stop"))]
pub async fn ping(_ctx: Context<'_>) -> Result<()> {
    Ok(())
}

/// Commence the Ping Cannon
#[poise::command(slash_command)]
pub async fn commence(
    ctx: Context<'_>,
    // TODO: refactor to work with Vec<Member>
    #[description = "Users to ping with the Ping Cannon"] users: String,
) -> Result<()> {
    let users = input_to_users(&users);
    match ctx
        .data()
        .ping_data
        .channel
        .send(PingWorkerMessage::Commence(
            ctx.serenity_context().http.clone(),
            ctx.channel_id(),
            users,
        ))
        .await
    {
        Ok(()) => _ = ctx.say("LOADING PING CANNON....").await,
        Err(e) => debug!("Error while sending Commence message: {:?}", e),
    };
    Ok(())
}

/// Remove users from running cannon
#[poise::command(slash_command)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "Users to remove from the Ping Cannon"] users: String,
) -> Result<()> {
    let users = input_to_users(&users);
    match ctx
        .data()
        .ping_data
        .channel
        .send(PingWorkerMessage::Remove(ctx.channel_id(), users))
        .await
    {
        Ok(()) => _ = ctx.say("Users removed from the targets.").await,
        Err(e) => debug!("Error while sending Remove message: {:?}", e),
    };

    Ok(())
}

/// Remove users from running cannon
#[poise::command(slash_command)]
pub async fn stop(ctx: Context<'_>) -> Result<()> {
    match ctx
        .data()
        .ping_data
        .channel
        .send(PingWorkerMessage::Stop(ctx.channel_id()))
        .await
    {
        Ok(()) => _ = ctx.say("The Ping Canon has stopped.").await,
        Err(e) => debug!("Error while sending Stop message: {:?}", e),
    };

    Ok(())
}

fn input_to_users(input: &str) -> HashSet<UserId> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"<@(\d+)>").unwrap();
    }

    RE.captures_iter(input)
        .map(|cap| {
            let id = &cap[1];
            let id: u64 = id.parse().unwrap();
            UserId(id)
        })
        .collect()
}

#[derive(Debug)]
struct PingTask {
    pub end_date: Instant,
    pub users: HashSet<UserId>,
}

impl PingTask {
    pub fn new(end_date: Instant, users: HashSet<UserId>) -> Self {
        Self { end_date, users }
    }

    pub fn is_done(&self) -> bool {
        Instant::now() > self.end_date
    }
}

#[derive(Debug)]
enum PingWorkerMessage {
    Commence(Arc<Http>, ChannelId, HashSet<UserId>),
    Remove(ChannelId, HashSet<UserId>),
    Stop(ChannelId),
}

#[derive(Debug)]
struct PingWorker {
    pings: HashMap<ChannelId, Mutex<PingTask>>,
    channel: Receiver<PingWorkerMessage>,
    http: Option<Arc<Http>>,
}

impl PingWorker {
    pub fn new(channel: Receiver<PingWorkerMessage>) -> Self {
        Self {
            pings: HashMap::new(),
            channel,
            http: None,
        }
    }

    #[allow(unused_must_use)]
    pub async fn work(&mut self) {
        let mut queue: Vec<ChannelId> = Vec::new();
        loop {
            self.pings.retain(|channel, ping_task| {
                let mut leave = true;
                if let Ok(ping_task) = ping_task.try_lock() {
                    if ping_task.is_done() {
                        leave = false;
                        queue.push(*channel);
                    }
                }
                leave
            });

            if let Some(http) = &self.http {
                for channel in &queue {
                    channel
                        .say(http, "The Ping Cannon shot enough shots.")
                        .await;
                }
                queue.clear();
            }

            for (channel, ping_task) in &self.pings {
                if let Ok(ping_task) = ping_task.try_lock() {
                    if let Some(http) = &self.http {
                        let usrs: String = ping_task
                            .users
                            .iter()
                            .map(|u| format!("<@!{}>", u.0))
                            .collect();
                        channel.say(http, format!("./ping {}", usrs)).await;
                    }
                }
            }

            self.handle_message().await;

            sleep(Duration::from_secs(1)).await;
        }
    }

    async fn handle_message(&mut self) {
        if let Ok(msg) = self.channel.try_recv() {
            use self::PingWorkerMessage::*;

            match msg {
                Commence(http, channel_id, users) => {
                    self.http = Some(http);

                    if let Some(ping_task) = self.pings.get_mut(&channel_id) {
                        let mut ping_task = ping_task.lock().await;
                        let usrs = &mut ping_task.users;
                        usrs.extend(users);
                    } else {
                        self.pings.insert(
                            channel_id,
                            Mutex::new(PingTask::new(Instant::now() + PING_TIMEOUT, users)),
                        );
                    }
                }
                Remove(channel_id, users) => {
                    let mut remove = false;
                    if let Some(ping_task) = self.pings.get_mut(&channel_id) {
                        let mut ping_task = ping_task.lock().await;
                        let usrs = &mut ping_task.users;
                        for usr in users {
                            usrs.remove(&usr);
                        }
                        if usrs.is_empty() {
                            remove = true;
                        }
                    }

                    if remove {
                        self.pings.remove(&channel_id);
                    }
                }
                Stop(channel_id) => {
                    self.pings.remove(&channel_id);
                }
            }
        }
    }
}
