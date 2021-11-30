use scylla::Session;
use scylla::transport::errors::QueryError;
use tracing::instrument;
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
}

impl Queries {
    #[instrument(skip(s), level = "debug")]
    pub async fn new(s: &Session) -> Result<Self, QueryError> {
        Ok(Queries {
            select_api_key: s.prepare("SELECT key,group FROM api_keys WHERE key = ?;").await?,
            select_ip_ban: s.prepare("SELECT ip, reason, date, end, ban, automated FROM ip_bans WHERE ip = ?;").await?,
            insert_ban_log:s.prepare("INSERT INTO bans_logs(id, start, end, target, ip, issuer, reason) VALUES (?, toTimestamp(now()), ?, ?, ?, ?, ?);").await?,
            insert_ip_ban: s.prepare("INSERT INTO ip_bans(ip, reason, date, end, ban, automated) VALUES (?, ?, toTimestamp(now()), null, ?, ?);").await?,
            insert_ip_ban_ttl: s.prepare("INSERT INTO ip_bans(ip, reason, date, end, ban, automated) VALUES (?, ?, toTimestamp(now()), ?, ?, ?) USING TTL ?;").await?,
            insert_server: s.prepare("INSERT INTO servers(id, description, ip, kind, label, properties, state) VALUES (?, ?, ?, ?, ?, ?, ?)").await?,
            delete_server: s.prepare("DELETE FROM servers WHERE id = ?;").await?
        })
    }
}