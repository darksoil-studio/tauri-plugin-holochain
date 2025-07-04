use std::{
    collections::{HashMap, HashSet},
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use async_std::stream::StreamExt;
use base64::Engine;
use holochain_client::AdminWebsocket;
use kitsune2_api::{AgentId, AgentInfoSigned, K2Error, SpaceId};
use kitsune2_core::Ed25519Verifier;
use kitsune_p2p_mdns::{mdns_create_broadcast_thread, mdns_kill_thread, mdns_listen};

pub async fn spawn_mdns_bootstrap(admin_port: u16) -> crate::Result<()> {
    tokio::spawn(async move {
        let wait_result = wait_until_admin_ws_is_available(admin_port).await;
        let Ok(admin_ws) = wait_result else {
            log::error!("Could not connect to websocket: {wait_result:?}");

            return ();
        };

        let mut spaces_listened_to: HashSet<SpaceId> = HashSet::new();
        let mut cells_ids_broadcasted: HashMap<(SpaceId, AgentId), std::sync::Arc<AtomicBool>> =
            HashMap::new();
        loop {
            let Ok(encoded_agent_infos) = admin_ws.agent_info(None).await else {
                continue;
            };

            let agent_infos: Vec<Arc<AgentInfoSigned>> = encoded_agent_infos
                .iter()
                .filter_map(|agent_info| {
                    AgentInfoSigned::decode(&Ed25519Verifier, agent_info.as_bytes()).ok()
                })
                .collect();

            let spaces: HashSet<SpaceId> = agent_infos
                .iter()
                .map(|agent_info| agent_info.space.clone())
                .collect();

            for space in spaces {
                if !spaces_listened_to.contains(&space) {
                    if let Err(err) = spawn_listen_to_space_task(space.clone(), admin_port).await {
                        log::error!("Error listening for mDNS space: {err:?}");
                        continue;
                    }
                    spaces_listened_to.insert(space);
                }
            }

            for agent_info in agent_infos {
                let cell_id = (agent_info.space.clone(), agent_info.agent.clone());
                if let Some(handle) = cells_ids_broadcasted.get(&cell_id) {
                    mdns_kill_thread(handle.to_owned());
                }
                // Broadcast by using Space as service type and Agent as service name
                let space_b64 =
                    base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(&agent_info.space[..]);
                let agent_b64 =
                    base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(&agent_info.agent[..]);

                // Broadcast rmp encoded agent_info_signed
                if let Ok(str_buffer) = agent_info.encode() {
                    let buffer = str_buffer.as_bytes();

                    let handle = mdns_create_broadcast_thread(space_b64, agent_b64, &buffer);
                    // store handle in self
                    cells_ids_broadcasted.insert(cell_id, handle);
                }
            }

            async_std::task::sleep(Duration::from_secs(5)).await;
        }
    });

    Ok(())
}

pub async fn spawn_listen_to_space_task(space: SpaceId, admin_port: u16) -> crate::Result<()> {
    let admin_ws = AdminWebsocket::connect(format!("127.0.0.1:{}", admin_port))
        .await
        .map_err(|err| {
            crate::Error::WebsocketConnectionError(format!(
                "Could not connect to websocket: {err:?}"
            ))
        })?;
    let space_b64 = base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(&space[..]);

    tokio::spawn(async move {
        let stream = mdns_listen(space_b64);
        tokio::pin!(stream);
        while let Some(maybe_response) = stream.next().await {
            match maybe_response {
                Ok(response) => {
                    log::debug!(
                        "Peer found via MDNS with service type {}, service name {} and address {}.",
                        response.service_type,
                        response.service_name,
                        response.addr
                    );
                    // Decode response
                    let maybe_agent_info_signed: Result<Arc<AgentInfoSigned>, K2Error> =
                        AgentInfoSigned::decode(&Ed25519Verifier, response.buffer.as_slice());
                    let Ok(remote_agent_info_signed) = maybe_agent_info_signed else {
                        log::error!("Failed to decode MDNS peer {:?}", maybe_agent_info_signed);
                        continue;
                    };
                    let Ok(encoded_agent_infos) = admin_ws.agent_info(None).await else {
                        continue;
                    };

                    let agent_infos: Vec<Arc<AgentInfoSigned>> = encoded_agent_infos
                        .iter()
                        .filter_map(|agent_info| {
                            AgentInfoSigned::decode(&Ed25519Verifier, agent_info.as_bytes()).ok()
                        })
                        .collect();

                    if agent_infos
                        .iter()
                        .find(|agent_info| remote_agent_info_signed.eq(&agent_info))
                        .is_none()
                    {
                        let Ok(encoded_agent_info) = remote_agent_info_signed.encode() else {
                            continue;
                        };
                        log::info!("Adding agent info {encoded_agent_info:?}");
                        if let Err(e) = admin_ws.add_agent_info(vec![encoded_agent_info]).await {
                            log::error!("Failed to store MDNS peer {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to get peers from MDNS {:?}", e);
                }
            }
        }
    });

    Ok(())
}

async fn wait_until_admin_ws_is_available(admin_port: u16) -> crate::Result<AdminWebsocket> {
    let mut retry_count = 0;
    loop {
        let connect_result = AdminWebsocket::connect(format!("127.0.0.1:{}", admin_port)).await;
        match connect_result {
            Ok(admin_ws) => {
                return Ok(admin_ws);
            }
            Err(err) => {
                log::error!("Could not connect to the admin interface: {}", err);
                async_std::task::sleep(Duration::from_millis(200)).await;

                retry_count += 1;
                if retry_count == 200 {
                    return Err(crate::Error::AdminWebsocketError(
                        "Can't connect to holochain".to_string(),
                    ));
                }
            }
        }
    }
}
