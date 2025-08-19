use clap::Parser;
use holochain::conductor::api::AppInfo;
use holochain_types::{
    app::{AppBundle, RoleSettings},
    dna::{AgentPubKey, AgentPubKeyB64},
};
use log::LevelFilter;
use std::path::PathBuf;
use std::{collections::HashMap, str::FromStr};
use tauri::{AppHandle, Context, Wry};
use tauri_plugin_holochain::{vec_to_locked, HolochainExt, HolochainPluginConfig, NetworkConfig};
use tauri_plugin_log::Target;
use url2::url2;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The path of the file tree to modify.
    pub happ_bundle_path: PathBuf,

    /// The password to protect the conductor by.
    #[clap(long)]
    pub password: Option<String>,

    /// The port where the UI server is running.
    #[clap(long)]
    pub ui_port: String,

    /// The admin port to bind the admin interface to.
    #[clap(long)]
    pub admin_port: Option<u16>,

    /// The agent key to install the app with.
    #[clap(long)]
    pub agent_key: Option<String>,

    /// The network seed to install the app with.
    #[clap(long)]
    pub network_seed: Option<String>,

    /// The signal URL to connect to.
    #[clap(long)]
    pub signal_url: Option<String>,

    /// The bootstrap URL to connect to.
    #[clap(long)]
    pub bootstrap_url: Option<String>,

    /// The directory where the conductor directories will be created.
    /// By default a new folder in the /tmp directory.
    #[clap(long)]
    pub conductor_dir: Option<PathBuf>,
}

fn log_level() -> LevelFilter {
    match std::env::var("RUST_LOG") {
        Ok(log) => LevelFilter::from_str(log.as_str()).expect("Invalid RUST_LOG value"),
        _ => LevelFilter::Warn,
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

    let mut network_config = NetworkConfig::default();

    match (args.signal_url, args.bootstrap_url) {
        (Some(signal_url), Some(bootstrap_url)) => {
            network_config.signal_url = url2!("{}", signal_url);
            network_config.bootstrap_url = url2!("{}", bootstrap_url);
        }
        (None, None) => {
            network_config.bootstrap_url = url2!("http://localhost:0000");
            network_config.signal_url = url2!("ws://localhost:0000");
        }
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
                .level(log_level())
                .clear_targets()
                .target(Target::new(tauri_plugin_log::TargetKind::Stdout))
                .build(),
        )
        .plugin(tauri_plugin_holochain::init(
            vec_to_locked(password.as_bytes().to_vec()),
            HolochainPluginConfig {
                network_config,
                holochain_dir: conductor_dir,
                admin_port: args.admin_port,
                mdns_discovery: true,
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
    let bytes = std::fs::read(app_bundle_path)?;
    let app_bundle = AppBundle::decode(&bytes)?;
    let app_id = app_bundle
        .clone()
        .into_inner()
        .manifest()
        .app_name()
        .to_string();
    let app_info = handle
        .holochain()?
        .install_app(app_id, app_bundle, roles_settings, agent_key, network_seed)
        .await?;
    log::info!("Installed app {app_info:?}");

    Ok(app_info)
}
