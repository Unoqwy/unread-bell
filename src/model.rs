use std::collections::HashMap;

use serde::{Deserialize, Serialize};

type DiscordId = String;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Notifications {
    dms: HashMap<DiscordId, DirectMessageNotifications>,
    groups: HashMap<DiscordId, GroupNotifications>,
    guilds: HashMap<DiscordId, GuildNotifications>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
struct DirectMessageNotifications {
    channel_id: String,
    unread_count: u32,
    last_message_id: String,
    username: String,
    discriminator: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
struct GroupNotifications {
    unread_count: u32,
    last_message_id: String,
    name: String,
    users: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
struct GuildNotifications {
    unread_count: u32,
    mention_count: u32,
    name: String,
}
