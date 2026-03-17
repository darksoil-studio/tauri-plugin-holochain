use base64::prelude::*;
use holochain_keystore::MetaLairClient;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HcAuthConfig {
    pub auth_server_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HcAuthStatus {
    Authorized,
    Pending,
    NotRegistered,
    Blocked,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct AuthFlowResult {
    pub status: HcAuthStatus,
    pub auth_material: Option<String>,
    pub agent_key: holochain_client::AgentPubKey,
    pub raw_ed25519_b64url: String,
}

fn auth_key_path(holochain_dir: &Path) -> PathBuf {
    holochain_dir.join("hc-auth-agent-key")
}

pub fn agent_pub_key_to_raw_ed25519_b64url(
    key: &holochain_client::AgentPubKey,
) -> String {
    let raw_32: &[u8] = key.get_raw_32();
    BASE64_URL_SAFE_NO_PAD.encode(raw_32)
}

pub async fn get_or_create_auth_key(
    keystore: &MetaLairClient,
    holochain_dir: &Path,
) -> crate::Result<holochain_client::AgentPubKey> {
    let key_path = auth_key_path(holochain_dir);

    if key_path.exists() {
        if let Ok(stored) = std::fs::read_to_string(&key_path) {
            let trimmed = stored.trim();
            if !trimmed.is_empty() {
                match holochain_client::AgentPubKey::try_from(trimmed) {
                    Ok(key) => {
                        log::info!("Reusing persisted hc-auth agent key");
                        return Ok(key);
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to parse persisted hc-auth key, generating new: {e:?}"
                        );
                    }
                }
            }
        }
    }

    log::info!("Generating new hc-auth agent key via Lair");
    let agent_pub_key = keystore
        .new_sign_keypair_random()
        .await
        .map_err(|e| crate::Error::LairError(e))?;

    let key_b64 = format!("{}", agent_pub_key);
    if let Err(e) = std::fs::write(&key_path, &key_b64) {
        log::error!("Failed to persist hc-auth agent key: {e}");
    }

    Ok(agent_pub_key)
}

pub async fn fetch_challenge(auth_server_url: &str) -> crate::Result<String> {
    let url = format!("{}/now", auth_server_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| crate::Error::HcAuthError(format!("GET /now failed: {e}")))?;

    if !resp.status().is_success() {
        return Err(crate::Error::HcAuthError(format!(
            "GET /now returned {}",
            resp.status()
        )));
    }

    resp.text()
        .await
        .map_err(|e| crate::Error::HcAuthError(format!("GET /now body read failed: {e}")))
}

pub async fn sign_challenge(
    keystore: &MetaLairClient,
    agent_key: &holochain_client::AgentPubKey,
    payload_b64url: &str,
) -> crate::Result<String> {
    let payload_bytes = BASE64_URL_SAFE_NO_PAD
        .decode(payload_b64url)
        .map_err(|e| crate::Error::HcAuthError(format!("Invalid payload base64url: {e}")))?;

    let mut pub_key_32 = [0u8; 32];
    pub_key_32.copy_from_slice(agent_key.get_raw_32());

    let signature = keystore
        .lair_client()
        .sign_by_pub_key(
            pub_key_32.into(),
            None,
            Arc::from(payload_bytes.as_slice()),
        )
        .await
        .map_err(|e| crate::Error::LairError(e))?;

    Ok(BASE64_URL_SAFE_NO_PAD.encode(&signature.0[..]))
}

pub fn build_auth_material(
    pubkey_b64url: &str,
    payload_b64url: &str,
    signature_b64url: &str,
) -> String {
    let auth_body = serde_json::json!({
        "pubKey": pubkey_b64url,
        "payload": payload_b64url,
        "signature": signature_b64url,
    });
    BASE64_STANDARD.encode(auth_body.to_string().as_bytes())
}

pub async fn try_authenticate(
    auth_server_url: &str,
    pubkey_b64url: &str,
    payload_b64url: &str,
    signature_b64url: &str,
) -> crate::Result<HcAuthStatus> {
    let url = format!("{}/authenticate", auth_server_url.trim_end_matches('/'));
    let auth_body = serde_json::json!({
        "pubKey": pubkey_b64url,
        "payload": payload_b64url,
        "signature": signature_b64url,
    });

    let client = reqwest::Client::new();
    let resp = client
        .put(&url)
        .header("Content-Type", "application/octet-stream")
        .body(auth_body.to_string())
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| {
            crate::Error::HcAuthError(format!("PUT /authenticate failed: {e}"))
        })?;

    match resp.status().as_u16() {
        200 => Ok(HcAuthStatus::Authorized),
        202 => Ok(HcAuthStatus::Pending),
        401 => Ok(HcAuthStatus::NotRegistered),
        403 => Ok(HcAuthStatus::Blocked),
        other => Err(crate::Error::HcAuthError(format!(
            "PUT /authenticate unexpected status: {other}"
        ))),
    }
}

pub async fn perform_auth_flow(
    keystore: &MetaLairClient,
    config: &HcAuthConfig,
    holochain_dir: &Path,
) -> crate::Result<AuthFlowResult> {
    let agent_key = get_or_create_auth_key(keystore, holochain_dir).await?;
    let raw_ed25519_b64url = agent_pub_key_to_raw_ed25519_b64url(&agent_key);

    log::info!(
        "hc-auth: Using agent key {}, raw Ed25519: {}",
        agent_key,
        raw_ed25519_b64url
    );

    let payload_b64url = match fetch_challenge(&config.auth_server_url).await {
        Ok(p) => p,
        Err(e) => {
            log::warn!("hc-auth: Could not reach auth server: {e}");
            return Ok(AuthFlowResult {
                status: HcAuthStatus::Failed(format!("Auth server unreachable: {e}")),
                auth_material: None,
                agent_key,
                raw_ed25519_b64url,
            });
        }
    };

    let signature_b64url =
        sign_challenge(keystore, &agent_key, &payload_b64url).await?;

    let status = match try_authenticate(
        &config.auth_server_url,
        &raw_ed25519_b64url,
        &payload_b64url,
        &signature_b64url,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            log::warn!("hc-auth: Authentication request failed: {e}");
            return Ok(AuthFlowResult {
                status: HcAuthStatus::Failed(format!("{e}")),
                auth_material: None,
                agent_key,
                raw_ed25519_b64url,
            });
        }
    };

    let auth_material = if status == HcAuthStatus::Authorized {
        let material =
            build_auth_material(&raw_ed25519_b64url, &payload_b64url, &signature_b64url);
        log::info!("hc-auth: Key authorized, auth material generated");
        Some(material)
    } else {
        log::info!("hc-auth: Key status = {:?}, no auth material", status);
        None
    };

    Ok(AuthFlowResult {
        status,
        auth_material,
        agent_key,
        raw_ed25519_b64url,
    })
}
