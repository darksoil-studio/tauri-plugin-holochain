use std::{
    collections::{HashMap, HashSet},
    sync::atomic::AtomicBool,
    time::Duration,
};

use async_std::stream::StreamExt;
use base64::Engine;
use holochain_client::AdminWebsocket;
use kitsune_p2p_mdns::{mdns_create_broadcast_thread, mdns_kill_thread, mdns_listen};
use kitsune_p2p_types::{
    agent_info::AgentInfoSigned,
    bin_types::{KitsuneAgent, KitsuneSpace},
    codec::{rmp_decode, rmp_encode},
};

pub async fn spawn_mdns_bootstrap(admin_port: u16) -> crate::Result<()> {
    let admin_ws = AdminWebsocket::connect(format!("localhost:{}", admin_port))
        .await
        .map_err(|err| {
            crate::Error::WebsocketConnectionError(format!(
                "Could not connect to websocket: {err:?}"
            ))
        })?;
    tokio::spawn(async move {
        let mut spaces_listened_to: HashSet<KitsuneSpace> = HashSet::new();
        let mut cells_ids_broadcasted: HashMap<
            (KitsuneSpace, KitsuneAgent),
            std::sync::Arc<AtomicBool>,
        > = HashMap::new();
        loop {
            let Ok(agent_infos) = admin_ws.agent_info(None).await else {
                continue;
            };

            let spaces: HashSet<KitsuneSpace> = agent_infos
                .iter()
                .map(|agent_info| agent_info.space.as_ref().clone())
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
                let cell_id = (
                    agent_info.space.as_ref().clone(),
                    agent_info.agent.as_ref().clone(),
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
                let mut buffer = Vec::new();
                if let Err(err) = rmp_encode(&mut buffer, &agent_info) {
                    log::error!("Error encoding buffer: {err:?}");
                    continue;
                };
                let handle = mdns_create_broadcast_thread(space_b64, agent_b64, &buffer);
                // store handle in self
                cells_ids_broadcasted.insert(cell_id, handle);
            }

            async_std::task::sleep(Duration::from_secs(5)).await;
        }
    });

    Ok(())
}

pub async fn spawn_listen_to_space_task(space: KitsuneSpace, admin_port: u16) -> crate::Result<()> {
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
                    log::debug!("Peer found via MDNS {:?}", response);
                    // Decode response
                    let maybe_agent_info_signed: Result<AgentInfoSigned, std::io::Error> =
                        rmp_decode(&mut &*response.buffer);
                    if let Err(e) = maybe_agent_info_signed {
                        log::error!("Failed to decode MDNS peer {:?}", e);
                        continue;
                    }
                    if let Ok(remote_agent_info_signed) = maybe_agent_info_signed {
                        let Ok(agent_infos) = admin_ws.agent_info(None).await else {
                            continue;
                        };

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
                            log::info!("Adding agent info {remote_agent_info_signed:?}");
                            if let Err(e) = admin_ws
                                .add_agent_info(vec![remote_agent_info_signed])
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
