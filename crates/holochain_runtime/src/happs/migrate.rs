use std::collections::HashMap;

use hc_zome_traits_utils::find_zomes_with_zome_trait;
use holochain::{
    conductor::api::error::ConductorApiError,
    core::DnaHash,
    prelude::{
        AppBundle, DnaBundle, DnaLocation, DnaModifiersOpt, RoleName, RoleSettings,
        RoleSettingsMap, YamlProperties,
    },
};
use holochain_client::{
    AllowedOrigins, AppInfo, CellId, CellInfo, ExternIO, InstalledAppId, SerializedBytes,
    ZomeCallTarget,
};

use crate::HolochainRuntime;

#[derive(Debug, thiserror::Error)]
pub enum MigrateAppError {
    #[error(transparent)]
    ConductorApiError(#[from] ConductorApiError),

    #[error("The given app was not found: {0}")]
    AppNotFoundError(String),

    #[error("Invalid DNA hash for role: {0}")]
    InvalidDnaHashError(String),

    #[error("Failed to find zome traits for role {0}: {1}")]
    GetZomeTraitsError(String, anyhow::Error),
}

// Installs the given new app while migrating the data from the existing one.
// The new app is installed under the same public key as the old one.
//
// For each DNA that didn't change its DNA hash, it uses `RoleSettings::UseExisting` to keep using the same cell.
// For each DNA that did change its DNA hash, calls the `migrate()` zome function on
// the new coordinator zomes that implement the migration zome trait.
pub async fn migrate_app(
    holochain_runtime: &HolochainRuntime,
    existing_app_id: InstalledAppId,
    new_app_id: InstalledAppId,
    new_app_bundle: AppBundle,
    new_roles_settings: Option<RoleSettingsMap>,
) -> crate::Result<AppInfo> {
    log::info!(
        "Migrating from old app {} to new app {}.",
        existing_app_id,
        new_app_id
    );
    let admin_ws = holochain_runtime.admin_websocket().await?;
    let apps = admin_ws.list_apps(None).await?;

    let Some(existing_app_info) = apps
        .into_iter()
        .find(|app| app.installed_app_id.eq(&existing_app_id))
    else {
        return Err(MigrateAppError::AppNotFoundError(existing_app_id))?;
    };

    let mut new_roles_settings = new_roles_settings.unwrap_or_default();

    let mut roles_settings = RoleSettingsMap::new();

    let mut migrated_roles_from_cells: HashMap<RoleName, CellId> = HashMap::new();

    // For every new role:
    // - Check if there was an existing provisioned cell
    //   - If there wasn't, use given roles settings
    //   - If there was:
    //     - Compute new dna and compare with existing
    //       - If the dna has not changed, add the RolesSettings::UseExisting
    //       - If the dna has changed, use given roles settings
    for new_role in new_app_bundle.manifest().app_roles() {
        let new_role_settings = new_roles_settings.remove(&new_role.name);

        if let Some(new_role_settings) = &new_role_settings {
            if let RoleSettings::UseExisting { cell_id } = new_role_settings {
                roles_settings.insert(
                    new_role.name,
                    RoleSettings::UseExisting {
                        cell_id: cell_id.clone(),
                    },
                );
                continue;
            }
        };

        let existing_cells = existing_app_info.cell_info.get(&new_role.name);

        let Some(existing_cell) =
            existing_cells
                .cloned()
                .unwrap_or_default()
                .iter()
                .find_map(|c| match c {
                    CellInfo::Provisioned(c) => Some(c.clone()),
                    _ => None,
                })
        else {
            if let Some(role_settings) = new_role_settings {
                roles_settings.insert(new_role.name, role_settings);
            }
            continue;
        };

        let new_modifiers = match &new_role_settings {
            Some(RoleSettings::Provisioned { modifiers, .. }) => match modifiers {
                Some(modifiers) => Some(dna_modifiers_yaml_to_bytes(modifiers.clone())?),
                None => None,
            },
            _ => None,
        };

        let Some(new_dna_hash) =
            dna_hash_for_app_bundle_role(&new_app_bundle, &new_role.name, new_modifiers).await?
        else {
            return Err(MigrateAppError::InvalidDnaHashError(new_role.name))?;
        };

        if new_dna_hash.eq(&existing_cell.cell_id.dna_hash()) {
            log::info!("Reusing role {}.", new_role.name);

            roles_settings.insert(
                new_role.name,
                RoleSettings::UseExisting {
                    cell_id: existing_cell.cell_id,
                },
            );
        } else {
            if let Some(role_settings) = new_role_settings {
                roles_settings.insert(new_role.name.clone(), role_settings);
            };
            migrated_roles_from_cells.insert(new_role.name, existing_cell.cell_id);
        }
    }

    let roles_settings = if roles_settings.is_empty() {
        None
    } else {
        Some(roles_settings)
    };

    let app_info = holochain_runtime
        .install_app(
            new_app_id,
            new_app_bundle,
            roles_settings,
            Some(existing_app_info.agent_pub_key),
            None,
        )
        .await?;

    let app_ws = holochain_runtime
        .app_websocket(app_info.installed_app_id.clone(), AllowedOrigins::Any)
        .await?;

    for (migrated_role, old_cell_id) in migrated_roles_from_cells {
        let Some(CellInfo::Provisioned(provisioned_cell)) = app_info
            .cell_info
            .get(&migrated_role)
            .cloned()
            .unwrap_or_default()
            .first()
            .cloned()
        else {
            log::info!(
                "Role {migrated_role} was marked for migration but was not created upon app installation."
            );
            continue;
        };

        let zomes = find_zomes_with_zome_trait(
            &admin_ws,
            &app_ws,
            &provisioned_cell.cell_id,
            migration_zome_trait::MIGRATION_ZOME_TRAIT_HASH,
        )
        .await
        .map_err(|err| MigrateAppError::GetZomeTraitsError(migrated_role.clone(), err))?;

        for zome in zomes {
            log::debug!("Migrating zome {zome} in role {migrated_role}...");
            app_ws
                .call_zome(
                    ZomeCallTarget::CellId(provisioned_cell.cell_id.clone()),
                    zome.clone(),
                    "migrate".into(),
                    ExternIO::encode(old_cell_id.clone())?,
                )
                .await?;
            log::info!("Successfully migrated zome {zome} in role {migrated_role}.");
        }
    }

    Ok(app_info)
}

// Computes the resulting DNA hashes for the given app
pub async fn dna_hash_for_app_bundle(
    app_bundle: &AppBundle,
    roles_settings: &Option<RoleSettingsMap>,
) -> crate::Result<Vec<DnaHash>> {
    let role_names: Vec<String> = app_bundle
        .manifest()
        .app_roles()
        .into_iter()
        .map(|r| r.name)
        .collect();

    let mut dna_hashes: Vec<DnaHash> = Vec::new();
    for role_name in role_names {
        let role_settings = match roles_settings {
            Some(s) => s.get(&role_name),
            None => None
        };
        let modifiers = role_settings
            .map(|s| match s {
                RoleSettings::Provisioned { modifiers, .. } => Some(modifiers.clone()),
                _ => None,
            })
            .flatten()
            .flatten();
        let bytes_modifiers = match modifiers {
            Some(m) => Some(dna_modifiers_yaml_to_bytes(m)?),
            None => None
        };

        let Some(dna_hash) =
            dna_hash_for_app_bundle_role(app_bundle, &role_name, bytes_modifiers).await?
        else {
            return Err(MigrateAppError::InvalidDnaHashError(role_name))?;
        };
        dna_hashes.push(dna_hash);
    }

    Ok(dna_hashes)
}

// Computes the resulting DNA hash for the given role and dna modifiers
pub async fn dna_hash_for_app_bundle_role(
    app_bundle: &AppBundle,
    role_name: &RoleName,
    dna_modifiers: Option<DnaModifiersOpt>,
) -> crate::Result<Option<DnaHash>> {
    let Some(role) = app_bundle
        .manifest()
        .app_roles()
        .into_iter()
        .find(|r| r.name.eq(role_name))
    else {
        return Ok(None);
    };

    let Some(DnaLocation::Bundled(path)) = role.dna.location else {
        return Ok(None);
    };

    let Some(dna_bundle_bytes) = app_bundle.bundled_resources().get(&path) else {
        return Ok(None);
    };

    let bundle = DnaBundle::decode(dna_bundle_bytes.inner())?;

    let (dna_file, _) = bundle.into_dna_file(DnaModifiersOpt::default()).await?;

    let dna_def = dna_file.dna_def().clone();

    let dna_def = if let Some(modifiers) = dna_modifiers {
        dna_def.update_modifiers(modifiers)
    } else {
        dna_def
    };

    Ok(Some(DnaHash::with_data_sync(&dna_def)))
}

fn dna_modifiers_yaml_to_bytes(
    modifiers: DnaModifiersOpt<YamlProperties>,
) -> crate::Result<DnaModifiersOpt> {
    let properties = match modifiers.properties {
        Some(properties) => {
            let bytes = SerializedBytes::try_from(properties)?;
            Some(bytes)
        }
        None => None,
    };

    Ok(DnaModifiersOpt {
        network_seed: modifiers.network_seed.clone(),
        properties,
    })
}
