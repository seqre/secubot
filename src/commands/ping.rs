use async_trait::async_trait;
use lazy_static::lazy_static;
use regex::Regex;
use serenity::{
    builder::CreateApplicationCommand,
    client::Context,
    http::client::Http,
    model::{
        id::{ChannelId, UserId},
        interactions::{
            application_command::{
                ApplicationCommandInteraction,
                ApplicationCommandInteractionDataOptionValue::String as OptString,
                ApplicationCommandOptionType,
            },
            InteractionResponseType,
        },
    },
};
use tokio::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
    task::JoinHandle,
    time::{sleep, Duration},
};

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::{
    commands::{Command, CommandResult},
    secubot::Secubot,
};

const PING_COMMAND: &'static str = "ping";
const PING_COMMAND_DESC: &'static str = "The Ping Cannon";
const PING_SUBCOMMAND_COMMENCE: &'static str = "commence";
const PING_SUBCOMMAND_REMOVE: &'static str = "remove";
const PING_SUBCOMMAND_STOP: &'static str = "stop";

const PING_CHANNEL_BUFFER: usize = 15;

struct PingWorker {
    pings: HashMap<ChannelId, Mutex<HashSet<UserId>>>,
    channel: Receiver<PingWorkerMessage>,
    http: Option<Arc<Http>>,
}

impl PingWorker {
    pub fn new(channel: Receiver<PingWorkerMessage>) -> Self {
        Self {
            pings: HashMap::new(),
            channel: channel,
            http: None,
        }
    }

    pub async fn work(&mut self) {
        loop {
            for (channel, users) in self.pings.iter() {
                if let Ok(users) = users.try_lock() {
                    if let Some(http) = &self.http {
                        let usrs: String = users.iter().map(|u| format!("<@!{}>", u)).collect();
                        channel.say(http, format!("./ping {}", usrs)).await;
                    }
                }
            }

            self.handle_message().await;

            if self.pings.len() == 0 {
                sleep(Duration::from_secs(1)).await;
            } else {
                sleep(Duration::from_millis(900)).await;
            }
        }
    }

    async fn handle_message(&mut self) {
        if let Ok(msg) = self.channel.try_recv() {
            use self::PingWorkerMessage::*;

            match msg {
                Commence(http, channel_id, users) => {
                    self.http = Some(http);

                    if let Some(usrs) = self.pings.get_mut(&channel_id) {
                        let mut usrs = usrs.lock().await;
                        usrs.extend(users);
                    } else {
                        self.pings.insert(channel_id, Mutex::new(users));
                    }
                }
                Remove(channel_id, users) => {
                    if let Some(usrs) = self.pings.get_mut(&channel_id) {
                        let mut usrs = usrs.lock().await;
                        for usr in users {
                            usrs.remove(&usr);
                        }
                    }
                }
                Stop(channel_id) => {
                    self.pings.remove(&channel_id);
                }
            }
        }
    }
}

#[derive(Debug)]
enum PingWorkerMessage {
    Commence(Arc<Http>, ChannelId, HashSet<UserId>),
    Remove(ChannelId, HashSet<UserId>),
    Stop(ChannelId),
}

pub struct PingCommand {
    _worker: JoinHandle<()>,
    channel: Sender<PingWorkerMessage>,
}

impl PingCommand {
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

    async fn commence(&self, http: Arc<Http>, channel_id: ChannelId, users: HashSet<UserId>) {
        self.channel
            .send(PingWorkerMessage::Commence(http.clone(), channel_id, users))
            .await;
    }

    async fn remove(&self, channel_id: &ChannelId, users: HashSet<UserId>) {
        self.channel
            .send(PingWorkerMessage::Remove(*channel_id, users))
            .await;
    }

    async fn stop(&self, channel_id: &ChannelId) {
        self.channel
            .send(PingWorkerMessage::Stop(*channel_id))
            .await;
    }

    fn input_to_users(input: &String) -> HashSet<UserId> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"<@!(\d+)>").unwrap();
        }

        RE.captures_iter(input)
            .map(|cap| {
                let id = &cap[1];
                let id: u64 = id.parse().unwrap();
                UserId(id)
            })
            .collect()
    }
}

#[async_trait]
impl Command for PingCommand {
    fn get_name(&self) -> &'static str {
        PING_COMMAND
    }

    fn add_application_command(&self, command: &mut CreateApplicationCommand) {
        command
            .description(PING_COMMAND_DESC)
            .create_option(|option| {
                option
                    .name(PING_SUBCOMMAND_COMMENCE)
                    .description("Commence the Ping Cannon")
                    .kind(ApplicationCommandOptionType::SubCommand)
                    .create_sub_option(|subopt| {
                        subopt
                            .name("users")
                            .description("users to ping")
                            .kind(ApplicationCommandOptionType::String)
                            .required(true)
                    })
            })
            .create_option(|option| {
                option
                    .name(PING_SUBCOMMAND_REMOVE)
                    .description("Remove users from running cannon")
                    .kind(ApplicationCommandOptionType::SubCommand)
                    .create_sub_option(|subopt| {
                        subopt
                            .name("users")
                            .description("users to remove")
                            .kind(ApplicationCommandOptionType::String)
                            .required(true)
                    })
            })
            .create_option(|option| {
                option
                    .name(PING_SUBCOMMAND_STOP)
                    .description("Stop the Ping Cannon")
                    .kind(ApplicationCommandOptionType::SubCommand)
            });
    }

    async fn handle(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
        secubot: &Secubot,
    ) -> CommandResult {
        let channel = command.channel_id;
        let subcommand = command
            .data
            .options
            .iter()
            .find(|x| x.kind == ApplicationCommandOptionType::SubCommand)
            .unwrap();
        let subcommand_name = subcommand.name.as_str();
        let args = &subcommand.options;

        let response_text = match subcommand_name {
            PING_SUBCOMMAND_STOP => {
                self.stop(&channel).await;
                "The Ping Canon has stopped."
            }
            name => {
                if let OptString(users) = args
                    .iter()
                    .find(|x| x.name == "users")
                    .expect("Expected users")
                    .resolved
                    .as_ref()
                    .expect("Expected users")
                {
                    let users = PingCommand::input_to_users(users);
                    match name {
                        PING_SUBCOMMAND_COMMENCE => {
                            let http = ctx.http.clone();
                            self.commence(http, channel, users).await;
                            "LOADING PING CANNON...."
                        }
                        PING_SUBCOMMAND_REMOVE => {
                            self.remove(&channel, users).await;
                            "Users removed from the targets."
                        }
                        &_ => {
                            unreachable! {}
                        }
                    }
                } else {
                    "Bad arguments... how?"
                }
            }
        };

        command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.content(response_text);
                        message
                    })
            })
            .await?;

        Ok(())
    }
}
