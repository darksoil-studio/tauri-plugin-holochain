use std::sync::Arc;

use holochain::{
    conductor::{
        config::{AdminInterfaceConfig, ConductorConfig, KeystoreConfig},
        interface::InterfaceDriver,
    },
    prelude::dependencies::kitsune_p2p_types::config::{
        tuning_params_struct::KitsuneP2pTuningParams, KitsuneP2pConfig, TransportConfig,
    },
};
use holochain_keystore::paths::KeystorePath;
use holochain_types::websocket::AllowedOrigins;
use url2::Url2;

use crate::filesystem::FileSystem;

pub fn conductor_config(
    fs: &FileSystem,
    admin_port: u16,
    lair_root: KeystorePath,
    bootstrap_url: Url2,
    signal_url: Url2,
    override_gossip_arc_clamping: Option<String>,
) -> ConductorConfig {
    let mut config = ConductorConfig::default();
    config.data_root_path = Some(fs.conductor_dir().into());
    config.keystore = KeystoreConfig::LairServerInProc {
        lair_root: Some(lair_root),
    };

    let mut network_config = KitsuneP2pConfig::default();

    let mut tuning_params = KitsuneP2pTuningParams::default();

    // if let Some(c) = override_gossip_arc_clamping {
    //     tuning_params.gossip_arc_clamping = c;
    // }

    network_config.tuning_params = Arc::new(tuning_params);

    network_config.bootstrap_service = Some(bootstrap_url);

    // tx5
    network_config.transport_pool.push(TransportConfig::WebRTC {
        signal_url: signal_url.to_string(),
    });

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
