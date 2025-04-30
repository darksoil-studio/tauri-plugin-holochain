use std::sync::Arc;

use holochain_keystore::MetaLairClient;
use lair_keystore::create_sql_pool_factory;
use lair_keystore_api::{
    config::{LairServerConfig, LairServerConfigInner},
    in_proc_keystore::InProcKeystore,
    types::SharedLockedArray,
    LairResult,
};
use tokio::io::AsyncWriteExt;

fn read_config(config_path: &std::path::Path) -> crate::Result<LairServerConfig> {
    let bytes = std::fs::read(config_path)?;

    let config =
        LairServerConfigInner::from_bytes(&bytes).map_err(|err| crate::Error::LairError(err))?;

    if let Err(e) = std::fs::read(config.clone().pid_file) {
        // Workaround xcode different containers
        std::fs::remove_dir_all(config_path.parent().unwrap())?;
        std::fs::create_dir_all(config_path.parent().unwrap())?;
        return Err(e)?;
    }

    Ok(Arc::new(config))
}

/// Spawn an in-process keystore backed by lair_keystore.
pub async fn spawn_lair_keystore_in_proc(
    config_path: std::path::PathBuf,
    passphrase: SharedLockedArray,
) -> LairResult<MetaLairClient> {
    let config = get_config(&config_path, passphrase.clone()).await?;

    let store_factory = create_sql_pool_factory(&config.store_file, &config.database_salt);

    let in_proc_keystore = InProcKeystore::new(config, store_factory, passphrase).await?;
    let lair_client = in_proc_keystore.new_client().await?;

    // now, just connect to it : )
    let k = MetaLairClient::from_client(lair_client).await?;
    Ok(k)
}

pub async fn get_config(
    config_path: &std::path::Path,
    passphrase: SharedLockedArray,
) -> LairResult<LairServerConfig> {
    match read_config(config_path) {
        Ok(config) => Ok(config),
        Err(_) => write_config(config_path, passphrase).await,
    }
}

pub async fn write_config(
    config_path: &std::path::Path,
    passphrase: SharedLockedArray,
) -> LairResult<LairServerConfig> {
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

    Ok(Arc::new(config))
}
