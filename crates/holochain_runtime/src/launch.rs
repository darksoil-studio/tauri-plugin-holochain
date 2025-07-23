use std::sync::Arc;

use async_std::sync::Mutex;
use keystore::spawn_lair_keystore_in_proc;
// use holochain_keystore::lair_keystore::spawn_lair_keystore_in_proc;
use lair_keystore::dependencies::hc_seed_bundle::SharedLockedArray;

use holochain::conductor::Conductor;

use crate::{filesystem::FileSystem, HolochainRuntime, HolochainRuntimeConfig};

mod config;
mod keystore;
mod mdns;
use mdns::spawn_mdns_bootstrap;

pub const DEVICE_SEED_LAIR_KEYSTORE_TAG: &'static str = "DEVICE_SEED";

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

    let config = config::conductor_config(
        &filesystem,
        admin_port,
        filesystem.keystore_dir().into(),
        config.network_config,
    );

    log::debug!("Built conductor config: {:?}.", config);

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
        .config(config)
        .passphrase(Some(passphrase))
        .with_keystore(keystore)
        .build()
        .await?;

    log::info!("Connected to the admin websocket");

    spawn_mdns_bootstrap(admin_port).await?;

    Ok(HolochainRuntime {
        filesystem,
        apps_websockets_auths: Arc::new(Mutex::new(Vec::new())),
        admin_port,
        conductor_handle,
    })
}
