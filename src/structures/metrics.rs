use prometheus::{Gauge, IntGauge};
use tracing::instrument;

pub struct Metrics {
    pub onlines: Box<IntGauge>
}


impl Metrics {
    #[instrument(name = "metrics_init")]
    pub fn new() -> Result<Self, prometheus::Error> {
        let r = prometheus::default_registry();
        let onlines = Box::new(IntGauge::new("skynet_onlines", "Online players")?);
        r.register(onlines.clone())?;

        Ok(Metrics{
            onlines
        })
    }
}