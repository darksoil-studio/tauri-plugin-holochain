use std::sync::Arc;

use holochain::conductor::{
    config::{AdminInterfaceConfig, ConductorConfig, KeystoreConfig},
    interface::InterfaceDriver,
};
use holochain_conductor_api::conductor::DpkiConfig;
use holochain_keystore::paths::KeystorePath;
use holochain_types::websocket::AllowedOrigins;
use kitsune_p2p_types::config::{
    tuning_params_struct::KitsuneP2pTuningParams, KitsuneP2pConfig, TransportConfig,
};
use url2::Url2;

use crate::{filesystem::FileSystem, launch::DEVICE_SEED_LAIR_KEYSTORE_TAG, WANNetworkConfig};

pub fn conductor_config(
    fs: &FileSystem,
    admin_port: u16,
    lair_root: KeystorePath,
    wan_network_config: Option<WANNetworkConfig>,
    local_signal_url: Option<Url2>,
    override_gossip_arc_clamping: Option<String>,
) -> ConductorConfig {
    let mut config = ConductorConfig::default();
    config.data_root_path = Some(fs.conductor_dir().into());
    config.keystore = KeystoreConfig::LairServerInProc {
        lair_root: Some(lair_root),
    };
    config.device_seed_lair_tag = Some(DEVICE_SEED_LAIR_KEYSTORE_TAG.into());
    config.dpki = DpkiConfig::disabled();
    
    let mut network_config = KitsuneP2pConfig::default();

    let mut tuning_params = KitsuneP2pTuningParams::default();

    if let Some(c) = override_gossip_arc_clamping {
        tuning_params.gossip_arc_clamping = c;
    }

    network_config.tuning_params = Arc::new(tuning_params);

    if let Some(wan_network_config) = wan_network_config {
        network_config.bootstrap_service = Some(wan_network_config.bootstrap_url);
        // WAN
        let webrtc_config = if wan_network_config.ice_servers_urls.is_empty() {
            None
        } else {
            Some(webrtc_config_from_ice_urls(
                wan_network_config.ice_servers_urls,
            ))
        };
        network_config.transport_pool.push(TransportConfig::WebRTC {
            webrtc_config,
            signal_url: wan_network_config.signal_url.to_string(),
        });
    }

    // LAN
    if let Some(local_signal_url) = local_signal_url {
        network_config.transport_pool.insert(0, TransportConfig::WebRTC {
        webrtc_config: None,
            signal_url: local_signal_url.to_string(),
        });
    }

    config.network = network_config;

    // TODO: uncomment when we can set a custom origin for holochain-client-rust
    // let mut origins: HashSet<String> = HashSet::new();
    // origins.insert(String::from("localhost")); // Compatible with the url of the main window: tauri://localhost
    // let allowed_origins = AllowedOrigins::Origins(origins);

    let allowed_origins = AllowedOrigins::Any;

    config.admin_interfaces = Some(vec![AdminInterfaceConfig {
        driver: InterfaceDriver::Websocket {
            port: admin_port,
            allowed_origins,
        },
    }]);

    config
}

fn webrtc_config_from_ice_urls(ice_server_urls: Vec<Url2>) -> serde_json::Value {
    let mut webrtc_config = serde_json::Map::new();
    let mut ice_servers = Vec::new();
    for url in ice_server_urls {
        let mut url_mapping = serde_json::Map::new();
        url_mapping.insert(
            String::from("urls"),
            serde_json::Value::Array(vec![serde_json::Value::String(url.to_string())]),
        );
        ice_servers.push(serde_json::Value::Object(url_mapping));
    }
    webrtc_config.insert(
        String::from("ice_servers"),
        serde_json::Value::Array(ice_servers),
    );
    serde_json::Value::Object(webrtc_config)
}
