-- EmmyLua declaration for unread-bell
-- Usage: clone the repo and symlink this file to ~/.config/unread-bell/docs.lua

---@class UnreadBell
---@field get_notifications fun():Notifications

---@class Notifications
---@field dms table<string, DirectMessageNotifications> Unread direct messages [user id -> info]
---@field groups table<string, GroupNotifications> Unread groups [group id -> info]
---@field guilds table<string, GuildNotifications> Unread guilds [guild id -> info]

---@class DirectMessageNotifications
---@field channel_id string Id of this DM channel
---@field unread_count number Number of unread messages
---@field last_message_id string Last received message in this DM channel
---@field username string Current recipient username
---@field descriminator string Current recipient discriminator

---@class GroupNotifications
---@field unread_count number Number of unread messages
---@field last_message_id string Last received message in this group
---@field name string Current group display name
---@field users string[] List of users in this group, by ids

---@class GuildNotifications
---@field unread_count number Number of unread messages
---@field mention_count number Number of unread mentions
---@field name string Current guild display name
