use std::{
    collections::{HashMap, HashSet},
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use async_std::stream::StreamExt;
use base64::Engine;
use holochain_client::AdminWebsocket;
use kitsune_p2p_mdns::{mdns_create_broadcast_thread, mdns_kill_thread, mdns_listen};
use kitsune2_core::Ed25519Verifier;
use kitsune2_api::{AgentId, AgentInfoSigned, K2Error, SpaceId};

pub async fn spawn_mdns_bootstrap(admin_port: u16) -> crate::Result<()> {
    let admin_ws = AdminWebsocket::connect(format!("localhost:{}", admin_port))
        .await
        .map_err(|err| {
            crate::Error::WebsocketConnectionError(format!(
                "Could not connect to websocket: {err:?}"
            ))
        })?;
    tokio::spawn(async move {
        let mut spaces_listened_to: HashSet<SpaceId> = HashSet::new();
        let mut cells_ids_broadcasted: HashMap<
            (SpaceId, AgentId),
            std::sync::Arc<AtomicBool>,
        > = HashMap::new();
        loop {
            let Ok(encoded_agent_infos) = admin_ws.agent_info(None).await else {
                continue;
            };
            // log::info!("hey {encoded_agent_infos:?}");

            let agent_infos: Vec<Arc<AgentInfoSigned>> = encoded_agent_infos
                .iter()
                .filter_map(|agent_info|
                    AgentInfoSigned::decode(
                    &Ed25519Verifier,
                  agent_info.as_bytes(),
                ).ok())
                .collect();

            // log::info!("hey2 {agent_infos:?}");
            let spaces: HashSet<SpaceId> = agent_infos.iter()
                .map(|agent_info| agent_info.space.clone())
                .collect();
            // log::info!("hey3 {spaces:?}");

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
                let cell_id = (
                    agent_info.space.clone(),
                    agent_info.agent.clone(),
                );
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
            // log::info!("broadcast {str_buffer:?}");
                   let buffer = str_buffer.as_bytes(); 
                    // if let Err(err) = rmp_encode(&mut buffer, &agent_info) {
                    //     log::error!("Error encoding buffer: {err:?}");
                    //     continue;
                    // };
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
    let admin_ws = AdminWebsocket::connect(format!("localhost:{}", admin_port))
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
                    let maybe_agent_info_signed: Result<Arc<AgentInfoSigned>, K2Error> = AgentInfoSigned::decode(
                        &Ed25519Verifier,
                        response.buffer.as_slice()
                    );
                    if let Err(e) = maybe_agent_info_signed {
                        log::error!("Failed to decode MDNS peer {:?}", e);
                        continue;
                    }
                    if let Ok(remote_agent_info_signed) = maybe_agent_info_signed {
                        let Ok(encoded_agent_infos) = admin_ws.agent_info(None).await else {
                            continue;
                        };
                        let agent_infos: Vec<Arc<AgentInfoSigned>> = encoded_agent_infos
                            .iter()
                            .filter_map(|agent_info|
                                AgentInfoSigned::decode(
                                &Ed25519Verifier,
                              agent_info.as_bytes(),
                            ).ok())
                            .collect();

                        if agent_infos
                            .iter()
                            .find(|agent_info| {
                                remote_agent_info_signed
                                    .agent
                                    .as_ref()
                                    .eq(agent_info.agent.as_ref())
                            })
                            .is_none()
                        {
                            let Ok(encoded_agent_info) = remote_agent_info_signed.encode() else {
                                continue;
                            };
                            log::info!("Adding agent info {encoded_agent_info:?}");
                            if let Err(e) = admin_ws
                                .add_agent_info(vec![encoded_agent_info])
                                .await
                            {
                                log::error!("Failed to store MDNS peer {:?}", e);
                            }
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
