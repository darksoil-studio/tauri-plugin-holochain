use holochain::prelude::{AppBundle, RoleSettingsMap};
use holochain_client::{AppInfo, AppStatusFilter, InstalledAppId};

use crate::HolochainRuntime;

use super::migrate::{dna_hash_for_app_bundle, migrate_app};

pub struct VersionedApp {
    pub(crate) holochain_runtime: HolochainRuntime,
    pub(crate) app_id_prefix: InstalledAppId,
}

impl VersionedApp {
    async fn app_id_for(
        &self,
        app_bundle: &AppBundle,
        roles_settings: &Option<RoleSettingsMap>,
    ) -> crate::Result<InstalledAppId> {
        let dna_hashes: Vec<String> = dna_hash_for_app_bundle(app_bundle, roles_settings)
            .await?
            .into_iter()
            .map(|h| h.to_string())
            .collect();

        Ok(format!("{}-{}", self.app_id_prefix, dna_hashes.join("-")))
    }

    pub async fn install_or_update(
        &self,
        app_bundle: AppBundle,
        roles_settings: Option<RoleSettingsMap>,
    ) -> crate::Result<()> {
        let app_id = self.app_id_for(&app_bundle, &roles_settings).await?;

        let admin_ws = self.holochain_runtime.admin_websocket().await?;

        let installed_apps = admin_ws.list_apps(Some(AppStatusFilter::Running)).await?;

        let app_is_already_installed = installed_apps
            .iter()
            .find(|app| app.installed_app_id.as_str().eq(&app_id))
            .is_some();

        if !app_is_already_installed {
            let previous_app = installed_apps
                .iter()
                .filter(|app| {
                    app.installed_app_id
                        .as_str()
                        .starts_with(&self.app_id_prefix)
                })
                .max_by_key(|app_info| app_info.installed_at);

            if let Some(previous_app) = previous_app {
                migrate_app(
                    &self.holochain_runtime,
                    previous_app.installed_app_id.clone(),
                    app_id,
                    app_bundle,
                    roles_settings,
                )
                .await?;

                // admin_ws
                //     .disable_app(previous_app.installed_app_id.clone())
                //     .await?;
            } else {
                self.holochain_runtime
                    .install_app(app_id, app_bundle, roles_settings, None, None)
                    .await?;
            }

            Ok(())
        } else {
            self.holochain_runtime
                .update_app_if_necessary(app_id, app_bundle)
                .await?;

            Ok(())
        }
    }

    pub async fn current_app(&self) -> crate::Result<Option<AppInfo>> {
        let mut app_versions = self.get_all_app_versions().await?;

        app_versions.sort_by_key(|app| app.installed_at);
        app_versions.reverse();

        Ok(app_versions.first().cloned())
    }

    async fn get_all_app_versions(&self) -> crate::Result<Vec<AppInfo>> {
        let admin_ws = self.holochain_runtime.admin_websocket().await?;

        let apps = admin_ws.list_apps(None).await?;

        let filtered_apps: Vec<AppInfo> = apps
            .into_iter()
            .filter(|app| {
                app.installed_app_id
                    .starts_with(format!("{}-", self.app_id_prefix).as_str())
            })
            .collect();

        log::error!("yooo {:?} {}", filtered_apps.len(), self.app_id_prefix);

        Ok(filtered_apps)
    }
}
