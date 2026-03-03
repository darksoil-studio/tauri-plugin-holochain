use std::path::Path;
use std::sync::Arc;

use async_std::sync::Mutex;
use holochain::conductor::{config::ConductorConfig, Conductor};
use keystore::spawn_lair_keystore_in_proc;
// use holochain_keystore::lair_keystore::spawn_lair_keystore_in_proc;
use lair_keystore::dependencies::hc_seed_bundle::SharedLockedArray;

use crate::{filesystem::FileSystem, HolochainRuntime, HolochainRuntimeConfig};

mod config;
mod keystore;
mod mdns;
use mdns::spawn_mdns_bootstrap;

pub const DEVICE_SEED_LAIR_KEYSTORE_TAG: &'static str = "DEVICE_SEED";

/// Write the conductor configuration to a YAML file in the app data directory
/// so that external tooling can discover the conductor's layout on disk.
fn write_conductor_config(
    app_data_dir: &Path,
    conductor_config: &ConductorConfig,
) -> std::io::Result<()> {
    let config_yaml_path = app_data_dir.join("conductor-config.yaml");
    let yaml = serde_yaml::to_string(conductor_config)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
    std::fs::write(&config_yaml_path, yaml)?;
    log::info!("Wrote conductor config to {}", config_yaml_path.display());
    Ok(())
}

/// Launch the holochain conductor in the background
pub(crate) async fn launch_holochain_runtime(
    passphrase: SharedLockedArray,
    config: HolochainRuntimeConfig,
) -> crate::error::Result<HolochainRuntime> {
    let filesystem = FileSystem::new(config.holochain_dir).await?;
    let admin_port = if let Some(admin_port) = config.admin_port {
        admin_port
    } else {
        portpicker::pick_unused_port().expect("No ports free")
    };

    let conductor_config = config::conductor_config(
        &filesystem,
        admin_port,
        filesystem.keystore_dir().into(),
        config.network_config,
    );

    log::debug!("Built conductor config: {:?}.", conductor_config);

    if let Err(err) = write_conductor_config(&filesystem.app_data_dir, &conductor_config) {
        log::error!("Failed to write conductor config to disk: {}", err);
    }

    let keystore =
        spawn_lair_keystore_in_proc(&filesystem.keystore_config_path(), passphrase.clone())
            .map_err(|err| crate::Error::LairError(err))?;

    log::info!("Keystore spawned successfully.");

    let seed_already_exists = keystore
        .lair_client()
        .get_entry(DEVICE_SEED_LAIR_KEYSTORE_TAG.into())
        .await
        .is_ok();

    if !seed_already_exists {
        keystore
            .lair_client()
            .new_seed(
                DEVICE_SEED_LAIR_KEYSTORE_TAG.into(),
                None, // Some(passphrase.clone()),
                true,
            )
            .await
            .map_err(|err| crate::Error::LairError(err))?;
    }

    let conductor_handle = Conductor::builder()
        .config(conductor_config)
        .passphrase(Some(passphrase))
        .with_keystore(keystore)
        .build()
        .await?;

    log::info!("Connected to the admin websocket");

    if config.mdns_discovery {
        spawn_mdns_bootstrap(admin_port).await?;
    }

    Ok(HolochainRuntime {
        filesystem,
        apps_websockets_auths: Arc::new(Mutex::new(Vec::new())),
        admin_port,
        conductor_handle,
    })
}
