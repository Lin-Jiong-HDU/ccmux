//! ccmuxd - session manager daemon

use ccmux::server::daemon::{Daemon, DaemonConfig};
use anyhow::Result;
use tracing_subscriber;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = DaemonConfig::default();

    // Run the daemon (blocking)
    tokio::runtime::Runtime::new()?
        .block_on(async {
            let daemon = Daemon::new(config)?;
            daemon.run().await
        })
}
