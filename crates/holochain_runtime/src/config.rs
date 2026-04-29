use holochain_conductor_api::conductor::NetworkConfig;
use std::path::PathBuf;

#[cfg(feature = "hc-auth")]
pub use crate::hc_auth::HcAuthConfig;

pub struct HolochainRuntimeConfig {
    /// The directory where the holochain files and databases will be stored in
    pub holochain_dir: PathBuf,

    // Holochain network config
    pub network_config: NetworkConfig,

    /// Force the conductor to run at this admin port
    pub admin_port: Option<u16>,

    /// Enable mDNS based discovery
    /// Useful to discover peers in the same LAN
    pub mdns_discovery: bool,

    /// hc-auth server configuration for authenticated bootstrap/relay networks
    #[cfg(feature = "hc-auth")]
    pub hc_auth: Option<HcAuthConfig>,

    /// Raw 32-byte seed to import into a fresh Lair keystore on launch.
    /// Consumed once during launch; the seed is inserted and the field cleared.
    #[cfg(feature = "hc-auth")]
    pub pending_import_seed: Option<Vec<u8>>,
}

impl HolochainRuntimeConfig {
    pub fn new(holochain_dir: PathBuf, network_config: NetworkConfig) -> Self {
        Self {
            holochain_dir,
            network_config,
            admin_port: None,
            mdns_discovery: false,
            #[cfg(feature = "hc-auth")]
            hc_auth: None,
            #[cfg(feature = "hc-auth")]
            pending_import_seed: None,
        }
    }

    pub fn admin_port(mut self, admin_port: u16) -> Self {
        self.admin_port = Some(admin_port);
        self
    }

    pub fn enable_mdns_discovery(mut self) -> Self {
        self.mdns_discovery = true;
        self
    }

    #[cfg(feature = "hc-auth")]
    pub fn with_hc_auth(mut self, config: HcAuthConfig) -> Self {
        self.hc_auth = Some(config);
        self
    }
}
