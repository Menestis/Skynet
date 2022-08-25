use std::sync::Arc;
use tracing::error;
use uuid::Uuid;
use crate::AppData;
use crate::messenger::servers_events::ServerEvent::PlayerCount;

pub async fn process_online_count(data: Arc<AppData>, uuid: Uuid, count: i32) {
    if !data.k8s.is_leader() {
        return;
    }
    data.player_count.write().await.insert(uuid, count);

    let online: i32 = data.player_count.read().await.values().sum();

    if let Err(e) = data.db.insert_setting("online_count", &online.to_string()).await {
        error!("{}", e);
    }
    if let Err(e) = data.msgr.send_event(&PlayerCount {
        count: online
    }).await {
        error!("{}", e);
    }
    data.metrics.onlines.set(online as i64);
}