use std::sync::Arc;

use sbd_server::{Config, SbdServer};
use url2::Url2;

pub async fn can_connect_to_signal_server(signal_url: Url2) -> std::io::Result<()> {
    let config = tx5_signal::SignalConfig {
        listener: false,
        allow_plain_text: true,
        ..Default::default()
    };
    let signal_url_str = if let Some(s) = signal_url.as_str().strip_suffix('/') {
        s
    } else {
        signal_url.as_str()
    };

    tx5_signal::SignalConnection::connect(signal_url_str, Arc::new(config)).await?;

    Ok(())
}

pub async fn run_local_signal_service(local_ip: String, port: u16) -> std::io::Result<SbdServer> {
    let mut config = Config::default();

    config.bind = vec![format!("{local_ip}:{port}")];
    log::info!("Running local signal service {:?}", config);

    let sig_hnd = SbdServer::new(config.into()).await?;
    Ok(sig_hnd)
}
