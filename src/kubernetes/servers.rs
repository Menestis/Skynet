use rand::Rng;
use tracing::instrument;

#[instrument]
pub fn generate_server_name(kind: &str, name: &str) -> String {
    format!("{}-{}-{}", kind, name, rand::thread_rng().gen_range(10000..99999))
}