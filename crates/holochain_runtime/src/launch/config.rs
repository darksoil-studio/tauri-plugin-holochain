use holochain::conductor::{
    config::{AdminInterfaceConfig, ConductorConfig, KeystoreConfig},
    interface::InterfaceDriver,
};
use holochain_conductor_api::conductor::DpkiConfig;
use holochain_keystore::paths::KeystorePath;
use holochain_types::websocket::AllowedOrigins;
use serde_json::json;
use url2::Url2;

use crate::{filesystem::FileSystem, launch::DEVICE_SEED_LAIR_KEYSTORE_TAG, NetworkConfig};

pub fn conductor_config(
    fs: &FileSystem,
    admin_port: u16,
    lair_root: KeystorePath,
    mut network_config: NetworkConfig,
    local_signal_url: Option<Url2>,
) -> ConductorConfig {
    let mut config = ConductorConfig::default();
    config.data_root_path = Some(fs.conductor_dir().into());
    config.keystore = KeystoreConfig::LairServerInProc {
        lair_root: Some(lair_root),
    };
    config.device_seed_lair_tag = Some(DEVICE_SEED_LAIR_KEYSTORE_TAG.into());
    config.dpki = DpkiConfig::disabled();

    // LAN
    if let Some(local_signal_url) = local_signal_url {
        // network_config.bootstrap_url = url2!("");
        network_config.signal_url = local_signal_url;
    }
    if let None = network_config.advanced {
        let advanced_config = json!({
          "k2Gossip": {
            "initiateIntervalMs": 100,
            "minInitiateIntervalMs": 100,
          },
          "tx5Transport": {
            "signalAllowPlainText": true,
          },
        });
        network_config.advanced = Some(advanced_config);
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

