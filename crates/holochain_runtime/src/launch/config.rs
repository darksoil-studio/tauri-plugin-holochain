use holochain::conductor::{
    config::{AdminInterfaceConfig, ConductorConfig, KeystoreConfig},
    interface::InterfaceDriver,
};
use holochain_keystore::paths::KeystorePath;
use holochain_types::websocket::AllowedOrigins;
// use holochain_conductor_api::conductor::DpkiConfig;
// use crate::launch::DEVICE_SEED_LAIR_KEYSTORE_TAG;
use crate::{filesystem::FileSystem, NetworkConfig};

pub fn conductor_config(
    fs: &FileSystem,
    admin_port: u16,
    lair_root: KeystorePath,
    mut network_config: NetworkConfig,
) -> ConductorConfig {
    let mut config = ConductorConfig::default();
    config.data_root_path = Some(fs.conductor_dir().into());
    config.keystore = KeystoreConfig::LairServerInProc {
        lair_root: Some(lair_root),
    };
    // config.device_seed_lair_tag = Some(DEVICE_SEED_LAIR_KEYSTORE_TAG.into());
    // config.dpki = DpkiConfig::disabled();

    if let None = network_config.advanced {
        let advanced_config = serde_json::json!({
            "tx5Transport": {
                "signalAllowPlainText": true,
            },
        });
        network_config.advanced = Some(advanced_config);
    }
    config.network = network_config;
    config.request_timeout_s = 30; // Much better than the default 60

    // TODO: uncomment when we can set a custom origin for holochain-client-rust
    // let mut origins: HashSet<String> = HashSet::new();
    // origins.insert(String::from("localhost")); // Compatible with the url of the main window: tauri://localhost
    // let allowed_origins = AllowedOrigins::Origins(origins);

    let allowed_origins = AllowedOrigins::Any;

    config.admin_interfaces = Some(vec![AdminInterfaceConfig {
        driver: InterfaceDriver::Websocket {
            port: admin_port,
            allowed_origins,
            danger_bind_addr: None,
        },
    }]);

    config
}
