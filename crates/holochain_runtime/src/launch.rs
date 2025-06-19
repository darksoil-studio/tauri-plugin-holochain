use std::sync::Arc;

use async_std::sync::Mutex;
use holochain_keystore::lair_keystore::spawn_lair_keystore_in_proc;
use lair_keystore::dependencies::hc_seed_bundle::SharedLockedArray;
use url2::url2;

use holochain::conductor::Conductor;

use crate::{
    filesystem::FileSystem,
    launch::signal::{can_connect_to_signal_server, run_local_signal_service},
    HolochainRuntime, HolochainRuntimeConfig,
};

mod config;
mod keystore;
mod mdns;
mod signal;
use mdns::spawn_mdns_bootstrap;

pub const DEVICE_SEED_LAIR_KEYSTORE_TAG: &'static str = "DEVICE_SEED";

// pub static RUNNING_HOLOCHAIN: RwLock<Option<RunningHolochainInfo>> = RwLock::const_new(None);

/// Launch the holochain conductor in the background
pub(crate) async fn launch_holochain_runtime(
    passphrase: SharedLockedArray,
    config: HolochainRuntimeConfig,
) -> crate::error::Result<HolochainRuntime> {
    // let mut lock = RUNNING_HOLOCHAIN.write().await;

    // if let Some(info) = lock.to_owned() {
    //     return Ok(info);
    // }

    let filesystem = FileSystem::new(config.holochain_dir).await?;
    let admin_port = if let Some(admin_port) = config.admin_port {
        admin_port
    } else {
        portpicker::pick_unused_port().expect("No ports free")
    };

    let mut maybe_local_signal_server: Option<(url2::Url2, sbd_server::SbdServer)> = None;

    let connect_result =
        can_connect_to_signal_server(config.network_config.signal_url.clone()).await;

    let run_local_signal_server = if let Err(err) = connect_result {
        log::warn!("Error connecting with the WAN signal server: {err:?}");
        config.fallback_to_lan_only
    } else {
        false
    };

    if run_local_signal_server {
        let my_local_ip = local_ip_address::local_ip().expect("Could not get local ip address");
        let port = portpicker::pick_unused_port().expect("No ports free");
        let signal_handle = run_local_signal_service(my_local_ip.to_string(), port).await?;

        let local_signal_url = url2!("ws://{my_local_ip}:{port}");

        maybe_local_signal_server = Some((local_signal_url.clone(), signal_handle));
    }

    let config = config::conductor_config(
        &filesystem,
        admin_port,
        filesystem.keystore_dir().into(),
        config.network_config,
        maybe_local_signal_server.as_ref().map(|s| s.0.clone()),
    );

    let keystore =
        spawn_lair_keystore_in_proc(&filesystem.keystore_config_path(), passphrase.clone())
            .await
            .map_err(|err| crate::Error::LairError(err))?;

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

    // *lock = Some(info.clone());

    Ok(HolochainRuntime {
        filesystem,
        apps_websockets_auths: Arc::new(Mutex::new(Vec::new())),
        admin_port,
        conductor_handle,
        _local_sbd_server: maybe_local_signal_server.map(|s| s.1),
    })
}
