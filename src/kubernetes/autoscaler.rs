use std::collections::HashMap;
use crate::database::servers::Server;
use crate::Kubernetes;


pub struct Autoscale {
    pub players_per_servers: i32,

}

impl Kubernetes {
    pub async fn tick_autoscale(&self) -> anyhow::Result<()> {
        // let servers = self.database.select_all_servers().await?;
        //
        // let mut d: HashMap<&str, (Autoscale, Vec<Server>)> = HashMap::new();
        //
        // for srv in servers {
        //     if let Some(kind) = self.database.get_cached_kind(&srv.kind) {
        //         if let Some(autoscale) = &kind.autoscale {
        //             let (_SELECT player, SUM(value) AS kills FROM statistics_by_key  WHERE key = 'PLAYER_KILLS' AND server_kind != 'enmu' GROUP BY player ALLOW FILTERING;, srvs) = d.entry(&kind.name).or_insert((serde_json::from_str(autoscale)?, Vec::new()));
        //             srvs.push(srv);
        //         }
        //     }
        // }
        //

        Ok(())
    }
}