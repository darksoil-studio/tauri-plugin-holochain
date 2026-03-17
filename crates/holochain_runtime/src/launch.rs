use std::path::Path;
use std::sync::Arc;

use async_std::sync::Mutex;
use holochain::conductor::{config::ConductorConfig, Conductor};
use keystore::spawn_lair_keystore_in_proc;
use lair_keystore::dependencies::hc_seed_bundle::SharedLockedArray;

use crate::{filesystem::FileSystem, HolochainRuntime, HolochainRuntimeConfig};

pub(crate) mod config;
pub(crate) mod keystore;
mod mdns;
use mdns::spawn_mdns_bootstrap;

pub const DEVICE_SEED_LAIR_KEYSTORE_TAG: &'static str = "DEVICE_SEED";

/// Write the conductor configuration to a YAML file in the app data directory
/// so that external tooling can discover the conductor's layout on disk.
pub(crate) fn write_conductor_config(
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

/// Launch the holochain conductor in the background.
///
/// Flow:
/// 1. Spawn Lair keystore in-proc (independent of conductor)
/// 2. Clone the MetaLairClient for independent access
/// 3. If hc-auth is configured, run the auth flow using the lair client
/// 4. Build conductor config (with auth material if available)
/// 5. Build conductor (passes the original keystore)
/// 6. Store cloned lair client in HolochainRuntime for restart capability
pub(crate) async fn launch_holochain_runtime(
    passphrase: SharedLockedArray,
    config: HolochainRuntimeConfig,
) -> crate::error::Result<HolochainRuntime> {
    let filesystem = FileSystem::new(config.holochain_dir.clone()).await?;
    let admin_port = if let Some(admin_port) = config.admin_port {
        admin_port
    } else {
        portpicker::pick_unused_port().expect("No ports free")
    };

    // Step 1: Spawn Lair keystore FIRST (decoupled from conductor)
    let keystore =
        spawn_lair_keystore_in_proc(&filesystem.keystore_config_path(), passphrase.clone())
            .map_err(|err| crate::Error::LairError(err))?;

    log::info!("Keystore spawned successfully.");

    // Step 2: Create device seed if needed
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
                None,
                true,
            )
            .await
            .map_err(|err| crate::Error::LairError(err))?;
    }

    // Step 3: Clone the lair client for independent use (survives conductor shutdown)
    let lair_client_clone = keystore.clone();

    // Step 4: If hc-auth is configured, run the auth flow before building conductor config
    let mut network_config = config.network_config;

    #[cfg(feature = "hc-auth")]
    let (hc_auth_status, hc_auth_agent_key, hc_auth_raw_ed25519_b64url) = {
        use crate::hc_auth::{self, HcAuthStatus};

        if let Some(ref hc_auth_config) = config.hc_auth {
            log::info!("hc-auth: Running auth flow against {}", hc_auth_config.auth_server_url);

            match hc_auth::perform_auth_flow(&lair_client_clone, hc_auth_config, &filesystem.app_data_dir).await {
                Ok(result) => {
                    if let Some(ref material) = result.auth_material {
                        network_config.base64_auth_material = Some(material.clone());
                        log::info!("hc-auth: Auth material set on network config");
                    }
                    (result.status, Some(result.agent_key), Some(result.raw_ed25519_b64url))
                }
                Err(e) => {
                    log::error!("hc-auth: Auth flow error: {e}");
                    (HcAuthStatus::Failed(format!("{e}")), None, None)
                }
            }
        } else {
            (HcAuthStatus::Failed("Not configured".into()), None, None)
        }
    };

    // Step 5: Build conductor config AFTER auth (so base64_auth_material is populated)
    let conductor_config = config::conductor_config(
        &filesystem,
        admin_port,
        filesystem.keystore_dir().into(),
        network_config,
    );

    log::debug!("Built conductor config: {:?}.", conductor_config);

    if let Err(err) = write_conductor_config(&filesystem.app_data_dir, &conductor_config) {
        log::error!("Failed to write conductor config to disk: {}", err);
    }

    // Step 6: Build conductor (passes original keystore)
    let conductor_handle = Conductor::builder()
        .config(conductor_config)
        .passphrase(Some(passphrase.clone()))
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
        lair_client: lair_client_clone,
        passphrase,
        #[cfg(feature = "hc-auth")]
        hc_auth_config: config.hc_auth,
        #[cfg(feature = "hc-auth")]
        hc_auth_status: Arc::new(std::sync::RwLock::new(hc_auth_status)),
        #[cfg(feature = "hc-auth")]
        hc_auth_agent_key: Arc::new(std::sync::RwLock::new(hc_auth_agent_key)),
        #[cfg(feature = "hc-auth")]
        hc_auth_raw_ed25519_b64url: Arc::new(std::sync::RwLock::new(hc_auth_raw_ed25519_b64url)),
    })
}
