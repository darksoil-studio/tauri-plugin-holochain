use std::sync::Arc;

use holochain_keystore::MetaLairClient;
use lair_keystore::create_sql_pool_factory;
use lair_keystore_api::{
    config::{LairServerConfig, LairServerConfigInner},
    in_proc_keystore::InProcKeystore,
    prelude::PwHashLimits,
    types::SharedLockedArray,
    LairResult,
};
use tokio::io::AsyncWriteExt;

fn read_config(config_path: &std::path::Path) -> LairResult<LairServerConfig> {
    let bytes = std::fs::read(config_path)?;

    let config = LairServerConfigInner::from_bytes(&bytes)?;

    Ok(Arc::new(config))
}

fn limits() -> PwHashLimits {
    if cfg!(any(target_os = "android", target_os = "ios")) {
        PwHashLimits::Interactive
    } else {
        PwHashLimits::Moderate
    }
}

/// Spawn an in-process keystore backed by lair_keystore.
pub fn spawn_lair_keystore_in_proc(
    config_path: &std::path::PathBuf,
    passphrase: SharedLockedArray,
) -> LairResult<MetaLairClient> {
    limits().with_exec(|| {
        holochain_util::tokio_helper::block_forever_on(async move {

            let config = get_config(config_path, passphrase.clone()).await?;

            log::debug!("Spawning lair keystore with config: {:?}.", config);

            let store_factory = create_sql_pool_factory(&config.store_file, &config.database_salt);

            log::debug!("Created store factory.");

            let in_proc_keystore = InProcKeystore::new(config, store_factory, passphrase).await?;

            log::debug!("Created in proc keystore.");

            let lair_client = in_proc_keystore.new_client().await?;

            log::debug!("Initialized lair client.");

            let k = MetaLairClient::from_client(lair_client).await?;
            log::debug!("Created meta lair client.");
            Ok(k)
        })
    })
}

pub async fn get_config(
    config_path: &std::path::Path,
    passphrase: SharedLockedArray,
) -> LairResult<LairServerConfig> {
    if !std::fs::exists(&config_path)? {
        write_config(config_path, passphrase).await?;
    }
    read_config(config_path)
}

pub async fn write_config(
    config_path: &std::path::Path,
    passphrase: SharedLockedArray,
) -> LairResult<LairServerConfig> {
    log::debug!("Creating new lair config.");
    let lair_root = config_path
        .parent()
        .ok_or_else(|| one_err::OneErr::from("InvalidLairConfigDir"))?;

    tokio::fs::DirBuilder::new()
        .recursive(true)
        .create(&lair_root)
        .await?;

    let config = LairServerConfigInner::new(lair_root, passphrase).await?;

    let mut config_f = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(config_path)
        .await?;

    config_f.write_all(config.to_string().as_bytes()).await?;
    config_f.shutdown().await?;
    drop(config_f);
    log::debug!("Written new lair config in {:?}.", config_path);

    Ok(Arc::new(config))
}
