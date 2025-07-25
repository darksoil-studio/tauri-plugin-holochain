use holochain_conductor_api::conductor::NetworkConfig;
use std::path::PathBuf;

pub struct HolochainRuntimeConfig {
    /// The directory where the holochain files and databases will be stored in
    pub holochain_dir: PathBuf,

    // Holochain network config
    pub network_config: NetworkConfig,

    /// Force the conductor to run at this admin port
    pub admin_port: Option<u16>,
}

impl HolochainRuntimeConfig {
    pub fn new(holochain_dir: PathBuf, network_config: NetworkConfig) -> Self {
        Self {
            holochain_dir,
            network_config,
            admin_port: None,
        }
    }

    pub fn admin_port(mut self, admin_port: u16) -> Self {
        self.admin_port = Some(admin_port);
        self
    }
}
