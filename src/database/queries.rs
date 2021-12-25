use scylla::Session;
use scylla::transport::errors::QueryError;
use tracing::{instrument, Instrument};
use scylla::statement::prepared_statement::PreparedStatement;

use PreparedStatement as PS;

pub struct Queries {
    pub select_api_key: PS,
    pub select_ip_ban: PS,
    pub insert_ban_log: PS,
    pub insert_ip_ban_ttl: PS,
    pub insert_ip_ban: PS,
    pub insert_server: PS,
    pub delete_server: PS,
    pub select_proxy_player_info: PS,
    pub insert_player: PS,
    pub insert_session: PS,
    pub update_player_proxy_online_info: PS,
    pub close_session: PS,
    pub close_player_session: PS,
    pub select_server_kind_by_key: PS,
    pub select_server_by_label: PS,
    pub update_server_key: PS,
    pub select_server_kind: PS,
    pub select_server_player_info: PS,
    pub update_player_server_online_info: PS,
    pub insert_stats: PS,
    pub select_all_servers: PS,
    pub update_server_state: PS,
    pub select_online_players_reduced_info: PS,
    pub select_online_players_count: PS,
    pub select_server_label_and_properties: PS,
    pub select_player_proxy: PS,
    pub select_server_label: PS,
    pub select_full_player_info: PS,
    pub insert_ban: PS,
    pub insert_ban_ttl: PS,
}

impl Queries {
    #[instrument(skip(s), level = "debug")]
    pub async fn new(s: &Session) -> Result<Self, QueryError> {
        Ok(Queries {
            select_api_key: s.prepare("SELECT key,group FROM api_keys WHERE key = ?;").await?,
            select_ip_ban: s.prepare("SELECT ip, reason, date, end, ban, automated FROM ip_bans WHERE ip = ?;").await?,
            insert_ban_log: s.prepare("INSERT INTO bans_logs(id, start, end, target, ip, issuer, reason) VALUES (?, toTimestamp(now()), ?, ?, ?, ?, ?);").await?,
            insert_ip_ban: s.prepare("INSERT INTO ip_bans(ip, reason, date, end, ban, automated) VALUES (?, ?, toTimestamp(now()), null, ?, ?);").await?,
            insert_ip_ban_ttl: s.prepare("INSERT INTO ip_bans(ip, reason, date, end, ban, automated) VALUES (?, ?, toTimestamp(now()), ?, ?, ?) USING TTL ?;").await?,
            insert_server: s.prepare("INSERT INTO servers(id, description, ip, key, kind, label, properties, state) VALUES (?, ?, ?, ?, ?, ?, ?, ?)").await?,
            delete_server: s.prepare("DELETE FROM servers WHERE id = ?;").await?,
            select_proxy_player_info: s.prepare("SELECT locale, groups, permissions, properties, ban, ban_reason, TTL(ban) AS ban_ttl FROM players WHERE uuid = ?;").await?,
            select_server_player_info: s.prepare("SELECT prefix, suffix, proxy, session, locale, groups, permissions, currency, premium_currency, blocked, inventory, properties FROM players WHERE uuid = ?;").await?,
            insert_player: s.prepare("INSERT INTO players(uuid, username, currency, premium_currency, locale, groups) VALUES (?, ?, 0, 0, ?, ['Default']);").await?,
            insert_session: s.prepare("INSERT INTO sessions(id, brand, ip, mods, player, version, start) VALUES (?, ?, ?, ?, ?, ?, dateOf(now()));").await?,
            update_player_proxy_online_info: s.prepare("UPDATE players SET proxy = ?, session = ?, username = ? WHERE uuid = ?;").await?,
            update_player_server_online_info: s.prepare("UPDATE players SET server = ? WHERE uuid = ?;").await?,
            close_session: s.prepare("UPDATE sessions SET end = toTimestamp(now()) WHERE id = ?;").await?,
            close_player_session: s.prepare("UPDATE players SET proxy = null, server = null, session = null WHERE uuid = ?;").await?,
            select_server_kind_by_key: s.prepare("SELECT kind FROM servers_by_key WHERE key = ?;").await?,
            select_server_by_label: s.prepare("SELECT id, description, ip, key, kind, label, properties, state FROM servers_by_label WHERE label = ?;").await?,
            update_server_key: s.prepare("UPDATE servers SET key = ? WHERE id = ?;").await?,
            select_server_kind: s.prepare("SELECT kind FROM servers WHERE id = ?;").await?,
            insert_stats: s.prepare("INSERT INTO statistics(player, session, timestamp, server_kind, key, value) VALUES (?, ? ,?, ?, ?, ?);").await?,
            select_all_servers: s.prepare("SELECT id, description, ip, key, kind, label, properties, state FROM servers;").await?,
            update_server_state: s.prepare("UPDATE servers SET state = ? WHERE id = ?;").await?,
            select_online_players_reduced_info: s.prepare("SELECT uuid, username, session, proxy, server FROM players_by_session;").await?,
            select_online_players_count: s.prepare("SELECT COUNT(*) FROM players_by_session;").await?,
            select_server_label_and_properties: s.prepare("SELECT label, properties FROM servers WHERE id = ?;").await?,
            select_player_proxy: s.prepare("SELECT proxy FROM players WHERE uuid = ?").await?,
            select_server_label: s.prepare("SELECT label FROM servers WHERE id = ?;").await?,
            select_full_player_info: s.prepare("SELECT uuid, ban, ban_reason, blocked, currency, premium_currency, friends, groups, inventory, locale, permissions, proxy, server, session, username, prefix, suffix FROM players WHERE uuid = ?").await?,
            insert_ban: s.prepare("UPDATE players SET ban = ?, ban_reason = ? WHERE uuid = ?;").await?,
            insert_ban_ttl: s.prepare("UPDATE players USING TTL ? SET ban = ?, ban_reason = ? WHERE uuid = ?;").await?,
        })
    }
}