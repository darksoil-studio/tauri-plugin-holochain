use url2::Url2;
use std::path::PathBuf;

pub struct WANNetworkConfig {
    pub bootstrap_url: Url2,
    pub signal_url: Url2,
    pub ice_servers_urls: Vec<Url2>,
}

pub enum GossipArcClamp {
    Full,
    Empty,
}

pub struct HolochainRuntimeConfig {
    /// The directory where the holochain files and databases will be stored in
    pub holochain_dir: PathBuf,

    /// If `None`, no WAN networking will take place, only mDNS based networking
    /// Peers in the same LAN will still be able to communicate with each other
    pub wan_network_config: Option<WANNetworkConfig>,

    /// Force the conductor to run at this admin port
    pub admin_port: Option<u16>,

    /// Force the conductor to always have a "full", or "empty" Gossip Arc for all DNAs.
    /// The Gossip Arc is the subsection of the DHT that you aim to store and serve to others.
    ///
    /// A Full Gossip Arc means that your peer will always try to hold the full DHT state,
    /// and serve it to others.
    ///
    /// An Empty Gossip Arc means that your peer will always go to the network to fetch DHT data,
    /// unless they authored it.
    pub gossip_arc_clamp: Option<GossipArcClamp>,

    /// Fallback to LAN only mode if the signal server configured in WANNetworkConfig can't be 
    /// reached at launch
    pub fallback_to_lan_only: bool
}

fn default_gossip_arc_clamp() -> Option<GossipArcClamp> {
    if cfg!(any(target_os = "android", target_os = "ios")) {
        Some(GossipArcClamp::Empty)
    } else {
        None
    }
}

impl HolochainRuntimeConfig {
    pub fn new(
        holochain_dir: PathBuf,
        wan_network_config: Option<WANNetworkConfig>,
    ) -> Self {
        Self {
            holochain_dir,
            wan_network_config,
            admin_port: None,
            gossip_arc_clamp: default_gossip_arc_clamp(),
            fallback_to_lan_only: true
        }
    }

    pub fn admin_port(mut self, admin_port: u16) -> Self {
        self.admin_port = Some(admin_port);
        self
    }

    pub fn gossip_arc_clamp(mut self, gossip_arc_clamp: GossipArcClamp) -> Self {
        self.gossip_arc_clamp = Some(gossip_arc_clamp);
        self
    }
}
