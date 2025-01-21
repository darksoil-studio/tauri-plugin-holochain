use anyhow::anyhow;
use clap::Parser;
use holochain_client::AppInfo;
use holochain_types::{
    app::{InstallAppPayload, RoleSettings},
    dna::{AgentPubKey, AgentPubKeyB64},
};
use lair_keystore::dependencies::sodoken::{BufRead, BufWrite};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::{AppHandle, Context, Wry};
use tauri_plugin_holochain::{HolochainExt, HolochainPluginConfig, WANNetworkConfig};
use url2::url2;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The path of the file tree to modify.
    pub happ_bundle_path: PathBuf,

    /// The bundle identifier for the Tauri app
    #[clap(long)]
    pub password: Option<String>,

    /// The bundle identifier for the Tauri app
    #[clap(long)]
    pub ui_port: String,

    /// The admin port to bind the admin interface to
    #[clap(long)]
    pub admin_port: Option<u16>,

    /// The bundle identifier for the Tauri app
    #[clap(long)]
    pub agent_key: Option<String>,

    /// The bundle identifier for the Tauri app
    #[clap(long)]
    pub network_seed: Option<String>,

    /// The bundle identifier for the Tauri app
    #[clap(long)]
    pub signal_url: Option<String>,

    /// The bundle identifier for the Tauri app
    #[clap(long)]
    pub bootstrap_url: Option<String>,

    /// The bundle identifier for the Tauri app
    #[clap(long)]
    pub conductor_dir: Option<PathBuf>,
}

fn vec_to_locked(mut pass_tmp: Vec<u8>) -> std::io::Result<BufRead> {
    match BufWrite::new_mem_locked(pass_tmp.len()) {
        Err(e) => {
            pass_tmp.fill(0);
            Err(e.into())
        }
        Ok(p) => {
            {
                let mut lock = p.write_lock();
                lock.copy_from_slice(&pass_tmp);
                pass_tmp.fill(0);
            }
            Ok(p.to_read())
        }
    }
}

fn main() {
    let args = Args::parse();

    let conductor_dir = match args.conductor_dir {
        Some(c) => c,
        None => {
            let tmp_dir =
                tempdir::TempDir::new("hc-pilot").expect("Could not create temporary directory");
            tmp_dir.into_path()
        }
    };

    let password = args.password.unwrap_or_default();

    let dev_url = url2!("http://localhost:{}", args.ui_port);

    let mut context: Context<Wry> = tauri::generate_context!();
    context.config_mut().build.dev_url = Some(dev_url.into());

    let wan_network_config = match (args.signal_url, args.bootstrap_url) {
        (Some(signal_url), Some(bootstrap_url)) => Some(WANNetworkConfig {
            signal_url: url2!("{}", signal_url),
            bootstrap_url: url2!("{}", bootstrap_url),
            ice_servers_urls: vec![],
        }),
        (None, None) => None,
        (Some(_), None) => {
            panic!("Invalid arguments: --signal-url was provided without --bootstrap-url")
        }
        (None, Some(_)) => {
            panic!("Invalid arguments: --bootstrap-url was provided without --signal-url")
        }
    };

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Warn)
                .build(),
        )
        .plugin(tauri_plugin_holochain::init(
            vec_to_locked(password.as_bytes().to_vec()).expect("Can't build passphrase"),
            HolochainPluginConfig {
                wan_network_config,
                holochain_dir: conductor_dir,
                admin_port: args.admin_port,
                gossip_arc_clamp: None,
                fallback_to_lan_only: true
            },
        ))
        .setup(|app| {
            let agent_key = match args.agent_key {
                Some(key) => {
                    let key_b64 = AgentPubKeyB64::from_b64_str(key.as_str())?;
                    Some(AgentPubKey::from(key_b64))
                }
                None => None,
            };
            let handle = app.handle();
            let result: anyhow::Result<()> = tauri::async_runtime::block_on(async move {
                let app_info = setup(
                    handle.clone(),
                    args.happ_bundle_path,
                    None,
                    agent_key,
                    args.network_seed,
                )
                .await?;

                handle
                    .holochain()?
                    .main_window_builder(
                        String::from("main"),
                        false,
                        Some(app_info.installed_app_id),
                        None,
                    )
                    .await?
                    .build()?;

                Ok(())
            });
            result?;

            Ok(())
        })
        .run(context)
        .expect("error while running tauri application");
}

async fn setup(
    handle: AppHandle,
    app_bundle_path: PathBuf,
    roles_settings: Option<HashMap<String, RoleSettings>>,
    agent_key: Option<AgentPubKey>,
    network_seed: Option<String>,
) -> anyhow::Result<AppInfo> {
    let admin_ws = handle.holochain()?.admin_websocket().await?;
    let app_info = admin_ws
        .install_app(InstallAppPayload {
            agent_key,
            roles_settings,
            network_seed,
            source: holochain_types::app::AppBundleSource::Path(app_bundle_path),
            installed_app_id: None,
            ignore_genesis_failure: false,
            allow_throwaway_random_agent_key: false,
        })
        .await
        .map_err(|err| anyhow!("Error installing the app: {err:?}"))?;
    log::info!("Installed app {app_info:?}");

    let response = admin_ws
        .enable_app(app_info.installed_app_id.clone())
        .await
        .map_err(|err| anyhow!("Error enabling the app: {err:?}"))?;

    Ok(response.app)
}
