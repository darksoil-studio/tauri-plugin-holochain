use std::collections::BTreeMap;

use holochain::prelude::AppStatus;
use holochain_client::{AdminWebsocket, ConductorApiError, InstalledAppId};
use holochain_conductor_api::CellInfo;
use holochain_types::prelude::{
    AppBundle, AppBundleError, AppManifest, CoordinatorBundle, CoordinatorManifest, DnaBundle,
    DnaError, DnaFile, DnaHash, RoleName, UpdateCoordinatorsPayload, ZomeDependency, ZomeError,
    ZomeManifest,
};

use mr_bundle::{error::MrBundleError, Bundle, ResourceBytes, ResourceIdentifier};

use crate::filesystem::FileSystemError;

#[derive(Debug, thiserror::Error)]
pub enum UpdateHappError {
    #[error(transparent)]
    AppBundleError(#[from] AppBundleError),

    #[error(transparent)]
    ZomeError(#[from] ZomeError),

    #[error(transparent)]
    MrBundleError(#[from] MrBundleError),

    #[error(transparent)]
    FileSystemError(#[from] FileSystemError),

    #[error(transparent)]
    DnaError(#[from] DnaError),

    #[error("ConductorApiError: `{0:?}`")]
    ConductorApiError(ConductorApiError),

    #[error("Error connecting to the websocket")]
    WebsocketError,

    #[error("The given app was not found: {0}")]
    AppNotFound(String),

    #[error("The role {0} was not found the app {1}")]
    RoleNotFound(RoleName, InstalledAppId),

    #[error("The resource {0} was not found the bundle {1}")]
    ResourceNotFound(String, String),
}

pub async fn update_app(
    admin_ws: &AdminWebsocket,
    app_id: String,
    bundle: AppBundle,
) -> Result<(), UpdateHappError> {
    log::info!(
        "Checking whether the coordinator zomes for app {} need to be updated",
        app_id
    );

    // Get the DNA def from the admin websocket
    let apps = admin_ws
        .list_apps(None)
        .await
        .map_err(|err| UpdateHappError::ConductorApiError(err))?;

    let mut app = apps
        .into_iter()
        .find(|app| app.installed_app_id.eq(&app_id))
        .ok_or(UpdateHappError::AppNotFound(app_id.clone()))?;

    let new_dna_files = resolve_dna_files(bundle).await?;

    let mut updated = false;

    for (role_name, new_dna_file) in new_dna_files {
        let cells = app
            .cell_info
            .swap_remove(&role_name)
            .ok_or(UpdateHappError::RoleNotFound(
                role_name.clone(),
                app.installed_app_id.clone(),
            ))?;

        for cell in cells {
            let mut zomes: Vec<ZomeManifest> = Vec::new();
            let mut resources: Vec<(String, ResourceBytes)> = Vec::new();

            let cell_id = match cell {
                CellInfo::Provisioned(c) => c.cell_id.clone(),
                CellInfo::Cloned(c) => c.cell_id.clone(),
                CellInfo::Stem(_c) => {
                    continue;
                }
            };
            let old_dna_def = admin_ws
                .get_dna_definition(cell_id.clone())
                .await
                .map_err(|err| UpdateHappError::ConductorApiError(err))?;

            for (zome_name, coordinator_zome) in new_dna_file.dna_def().coordinator_zomes.iter() {
                let deps = coordinator_zome
                    .clone()
                    .erase_type()
                    .dependencies()
                    .to_vec();
                let dependencies = deps
                    .into_iter()
                    .map(|name| ZomeDependency { name })
                    .collect();

                if let Some(old_zome_def) = old_dna_def
                    .coordinator_zomes
                    .iter()
                    .find(|(zome, _)| zome.eq(&zome_name))
                {
                    if !old_zome_def
                        .1
                        .wasm_hash(&zome_name)?
                        .eq(&coordinator_zome.wasm_hash(&zome_name)?)
                    {
                        log::info!("Updating coordinator zome {zome_name} for role {role_name}");
                        zomes.push(ZomeManifest {
                            name: zome_name.clone(),
                            hash: None,
                            path: zome_name.0.to_string(),
                            dependencies: Some(dependencies),
                        });
                        let wasm = new_dna_file.get_wasm_for_zome(&zome_name)?;
                        resources
                            .push((zome_name.0.to_string(), wasm.clone().code().to_vec().into()));
                    }
                } else {
                    log::info!("Adding new coordinator zome {zome_name} for role {role_name}");
                    zomes.push(ZomeManifest {
                        name: zome_name.clone(),
                        hash: None,
                        path: zome_name.0.to_string(),
                        dependencies: Some(dependencies),
                    });
                    let wasm = new_dna_file.get_wasm_for_zome(&zome_name)?;
                    resources.push((zome_name.0.to_string(), wasm.clone().code().to_vec().into()));
                }
            }

            if !zomes.is_empty() {
                let source: CoordinatorBundle =
                    Bundle::new(CoordinatorManifest { zomes }, resources)?.into();
                let req = UpdateCoordinatorsPayload {
                    cell_id,
                    source: holochain_types::prelude::CoordinatorSource::Bundle(Box::new(source)),
                };

                admin_ws
                    .update_coordinators(req)
                    .await
                    .map_err(|err| UpdateHappError::ConductorApiError(err))?;
                updated = true;
            }
        }
    }

    if updated {
        if let AppStatus::Enabled = app.status {
            admin_ws
                .disable_app(app_id.clone())
                .await
                .map_err(|err| UpdateHappError::ConductorApiError(err))?;
            admin_ws
                .enable_app(app_id.clone())
                .await
                .map_err(|err| UpdateHappError::ConductorApiError(err))?;
        }
        log::info!("Updated app {app_id:?}");
    }

    Ok(())
}

async fn resolve_dna_files(
    app_bundle: AppBundle,
) -> Result<BTreeMap<RoleName, DnaFile>, UpdateHappError> {
    let mut dna_files: BTreeMap<RoleName, DnaFile> = BTreeMap::new();

    let bundle = app_bundle.into_inner();

    for app_role in bundle.manifest().app_roles() {
        if let Some(location) = app_role.dna.path {
            let (dna_def, _) = resolve_location(&bundle, &location).await?;

            dna_files.insert(app_role.name.clone(), dna_def);
        }
    }

    Ok(dna_files)
}

async fn resolve_location(
    app_bundle: &Bundle<AppManifest>,
    location: &ResourceIdentifier,
) -> Result<(DnaFile, DnaHash), UpdateHappError> {
    let bytes = app_bundle
        .get_resource(location)
        .ok_or(UpdateHappError::ResourceNotFound(
            location.clone(),
            app_bundle.manifest().app_name().to_string(),
        ))?;
    let dna_bundle: DnaBundle = mr_bundle::Bundle::unpack(bytes.as_ref())?.into();
    let (dna_file, original_hash) = dna_bundle.into_dna_file(Default::default()).await?;
    Ok((dna_file, original_hash))
}
