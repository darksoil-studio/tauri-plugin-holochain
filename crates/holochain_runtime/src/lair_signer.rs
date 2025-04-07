use async_trait::async_trait;
use holochain::prelude::ZomeCallParams;
use holochain_client::{AgentSigner, ExternIO};
use holochain_conductor_api::ZomeCallParamsSigned;
use holochain_types::prelude::{AgentPubKey, CapSecret, CellId, Signature};
use lair_keystore_api::LairClient;
use std::sync::Arc;

pub struct LairAgentSignerWithProvenance {
    lair_client: Arc<LairClient>,
}

impl LairAgentSignerWithProvenance {
    pub fn new(lair_client: Arc<LairClient>) -> Self {
        Self { lair_client }
    }
}

#[async_trait]
impl AgentSigner for LairAgentSignerWithProvenance {
    async fn sign(
        &self,
        _cell_id: &CellId,
        provenance: AgentPubKey,
        data_to_sign: Arc<[u8]>,
    ) -> anyhow::Result<Signature> {
        let public_key: [u8; 32] = provenance.get_raw_32().try_into()?;

        let signature = self
            .lair_client
            .sign_by_pub_key(public_key.into(), None, data_to_sign)
            .await?;

        Ok(Signature(*signature.0))
    }

    fn get_provenance(&self, cell_id: &CellId) -> Option<AgentPubKey> {
        Some(cell_id.agent_pubkey().clone())
    }

    /// Not used with Lair signing. If you have access to Lair then you don't need to prove you
    // are supposed to have access to a specific key pair.
    fn get_cap_secret(&self, _cell_id: &CellId) -> Option<CapSecret> {
        None
    }
}


/// Signs an unsigned zome call with the given LairClient
pub(crate) async fn sign_zome_call_with_client(
    params: ZomeCallParams,
    client: &LairClient,
) -> crate::Result<ZomeCallParamsSigned> {
    let pub_key = params.provenance.clone();
    let mut pub_key_2 = [0; 32];
    pub_key_2.copy_from_slice(pub_key.get_raw_32());

    let (bytes, bytes_hash) = params.serialize_and_hash()?;
    let signature = client
        .sign_by_pub_key(pub_key_2.into(), None, bytes_hash.into())
        .await
        .map_err(|err| crate::Error::LairError(err))?;

    let signature = Signature(*signature.0);

    Ok(ZomeCallParamsSigned {
        bytes: ExternIO(bytes),
        signature,
    })
}
