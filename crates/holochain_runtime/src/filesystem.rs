use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use std::{fs, io::Write};

use holochain::prelude::*;
use holochain_types::web_app::WebAppBundle;
use mr_bundle::error::MrBundleError;
use zip::result::ZipError;

pub struct FileSystem {
    pub app_data_dir: PathBuf,
    pub bundle_store: BundleStore,
}

impl FileSystem {
    pub async fn new(app_data_dir: PathBuf) -> crate::Result<FileSystem> {
        let bundle_store_path = app_data_dir.join("bundles");
        fs::create_dir_all(bundle_store_path.clone())?;
        let bundle_store = BundleStore::new(bundle_store_path)?;

        let fs = FileSystem {
            app_data_dir,
            bundle_store,
        };

        fs::create_dir_all(fs.keystore_dir())?;
        fs::create_dir_all(fs.conductor_dir())?;

        Ok(fs)
    }

    pub fn keystore_dir(&self) -> PathBuf {
        self.app_data_dir.join("keystore")
    }

    pub fn keystore_config_path(&self) -> PathBuf {
        self.keystore_dir().join("lair-keystore-config.yaml")
    }

    pub fn keystore_store_path(&self) -> PathBuf {
        self.keystore_dir().join("store_file")
    }

    pub fn conductor_dir(&self) -> PathBuf {
        self.app_data_dir.join("conductor")
    }
}

pub struct BundleStore {
    path: PathBuf,
    pub installed_apps_store: InstalledAppsStore,
}

impl BundleStore {
    fn new(path: PathBuf) -> crate::Result<Self> {
        let installed_apps_store = InstalledAppsStore::new(path.join("installed-apps.json"))?;

        let bundle_store = BundleStore {
            path,
            installed_apps_store,
        };
        fs::create_dir_all(bundle_store.happ_bundle_store().path)?;
        fs::create_dir_all(bundle_store.ui_store().path)?;

        Ok(bundle_store)
    }

    pub fn happ_bundle_store(&self) -> AppBundleStore {
        AppBundleStore {
            path: self.path.join("happs"),
        }
    }

    fn ui_store(&self) -> UiStore {
        UiStore {
            path: self.path.join("uis"),
        }
    }

    pub fn get_ui_path(&self, app_id: &InstalledAppId) -> crate::Result<PathBuf> {
        let installed_apps = self.installed_apps_store.get()?;

        let Some(installed_app_info) = installed_apps.get(app_id) else {
            return Err(crate::Error::AppDoesNotExist(app_id.clone()));
        };
        let Some(installed_web_app_info) = &installed_app_info.web_app_info else {
            return Err(crate::Error::AppDoesNotHaveUIError(app_id.clone()));
        };

        let path = self
            .ui_store()
            .get_path_for_ui_with_hash(&installed_web_app_info.ui_hash);

        Ok(path)
    }

    pub fn store_happ_bundle(
        &self,
        app_id: InstalledAppId,
        app_bundle: &AppBundle,
    ) -> crate::Result<()> {
        let happ_bundle_hash = self.happ_bundle_store().store_app_bundle(&app_bundle)?;
        self.installed_apps_store.update(|installed_apps| {
            installed_apps.insert(
                app_id.clone(),
                InstalledAppInfo {
                    happ_bundle_hash: happ_bundle_hash.clone(),
                    web_app_info: None,
                },
            );
        })?;

        Ok(())
    }

    pub fn web_app_bundle_hash(web_app_bundle: &WebAppBundle) -> crate::Result<String> {
        let web_happ_bundle_hash = sha256::digest(web_app_bundle.encode()?);
        Ok(web_happ_bundle_hash)
    }

    pub async fn store_web_happ_bundle(
        &self,
        app_id: InstalledAppId,
        web_app_bundle: &WebAppBundle,
    ) -> crate::Result<()> {
        let web_happ_bundle_hash = Self::web_app_bundle_hash(&web_app_bundle)?;

        let happ_bundle = web_app_bundle.happ_bundle().await?;
        let happ_bundle_hash = self.happ_bundle_store().store_app_bundle(&happ_bundle)?;
        let ui_hash = self
            .ui_store()
            .extract_and_store_ui(&web_app_bundle)
            .await?;

        self.installed_apps_store.update(move |installed_apps| {
            installed_apps.insert(
                app_id.clone(),
                InstalledAppInfo {
                    happ_bundle_hash: happ_bundle_hash.clone(),
                    web_app_info: Some(InstalledWebAppInfo {
                        web_happ_bundle_hash: web_happ_bundle_hash.clone(),
                        ui_hash: ui_hash.clone(),
                    }),
                },
            );
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InstalledWebAppInfo {
    pub ui_hash: String,
    pub web_happ_bundle_hash: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InstalledAppInfo {
    pub happ_bundle_hash: String,
    pub web_app_info: Option<InstalledWebAppInfo>,
}

pub type InstalledAppsInfo = HashMap<String, InstalledAppInfo>;

pub struct InstalledAppsStore {
    json_config_path: PathBuf,
    installed_apps: Arc<RwLock<InstalledAppsInfo>>,
}

impl InstalledAppsStore {
    fn new(json_config_path: PathBuf) -> crate::Result<Self> {
        let apps = if json_config_path.exists() {
            let s = std::fs::read_to_string(json_config_path.clone())?;

            let apps: HashMap<String, InstalledAppInfo> = serde_json::from_str(s.as_str())?;
            apps
        } else {
            let mut file = std::fs::File::create(json_config_path.clone())?;

            let apps: HashMap<String, InstalledAppInfo> = HashMap::new();

            let data = serde_json::to_string(&apps)?;

            file.write(&data.as_bytes())?;
            apps
        };

        Ok(Self {
            json_config_path,
            installed_apps: Arc::new(RwLock::new(apps)),
        })
    }

    pub fn get(&self) -> crate::Result<InstalledAppsInfo> {
        let apps = self
            .installed_apps
            .read()
            .map_err(|err| crate::Error::LockError(format!("{err:?}")))?;
        Ok(apps.clone())
    }

    pub fn update<F>(&self, update_fn: F) -> crate::Result<()>
    where
        F: Fn(&mut InstalledAppsInfo) -> (),
    {
        let mut write_lock = self
            .installed_apps
            // .write_arc()
            .write()
            .map_err(|err| crate::Error::LockError(format!("{err:?}")))?;

        update_fn(&mut write_lock);

        let data = serde_json::to_string(&write_lock.clone())?;

        std::fs::write(self.json_config_path.clone(), data)?;

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FileSystemError {
    #[error(transparent)]
    MrBundleError(#[from] MrBundleError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    ZipError(#[from] ZipError),
}

pub struct UiStore {
    path: PathBuf,
}

impl UiStore {
    pub async fn extract_and_store_ui(
        &self,
        web_app: &WebAppBundle,
    ) -> Result<String, FileSystemError> {
        let ui_bytes = web_app.web_ui_zip_bytes().await?;

        let hash = sha256::digest(ui_bytes.to_vec());

        let ui_folder_path = self.path.join(&hash);

        if ui_folder_path.exists() {
            fs::remove_dir_all(&ui_folder_path)?;
        }

        fs::create_dir_all(&ui_folder_path)?;

        let ui_zip_path = self.path.join("ui.zip");

        fs::write(ui_zip_path.clone(), ui_bytes.into_owned().into_inner())?;

        let file = std::fs::File::open(ui_zip_path.clone())?;
        unzip_file(file, ui_folder_path)?;

        fs::remove_file(ui_zip_path)?;

        Ok(hash)
    }

    fn get_path_for_ui_with_hash(&self, ui_hash: &String) -> PathBuf {
        self.path.join(ui_hash)
    }
}

pub struct AppBundleStore {
    path: PathBuf,
}

impl AppBundleStore {
    pub fn app_bundle_hash(app_bundle: &AppBundle) -> crate::Result<String> {
        let bytes = app_bundle.encode()?;
        let hash = sha256::digest(bytes);
        Ok(hash)
    }

    pub fn app_bundle_path(&self, app_bundle: &AppBundle) -> crate::Result<PathBuf> {
        Ok(self
            .path
            .join(format!("{}.happ", Self::app_bundle_hash(app_bundle)?)))
    }

    // pub fn get_webapp(
    //     &self,
    //     web_app_entry_hash: &EntryHash,
    // ) -> crate::Result<Option<WebAppBundle>> {
    //     let path = self.webhapp_path(web_app_entry_hash);

    //     if path.exists() {
    //         let bytes = fs::read(self.webhapp_package_path(&web_app_entry_hash))?;
    //         let web_app = WebAppBundle::decode(bytes.as_slice())?;

    //         return Ok(Some(web_app));
    //     } else {
    //         return Ok(None);
    //     }
    // }

    pub fn store_app_bundle(&self, app_bundle: &AppBundle) -> crate::Result<String> {
        let bytes = app_bundle.encode()?;
        let hash = sha256::digest(&bytes);
        let path = self.path.join(format!("{}.happ", hash));

        let mut file = std::fs::File::create(path)?;
        file.write_all(bytes.as_slice())?;

        Ok(hash)
    }
}

pub fn unzip_file(reader: std::fs::File, outpath: PathBuf) -> Result<(), FileSystemError> {
    let mut archive = zip::ZipArchive::new(reader)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).expect("Failed to archive by index");
        let outpath = match file.enclosed_name() {
            Some(path) => outpath.join(path).to_owned(),
            None => continue,
        };

        if (&*file.name()).ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p)?;
                }
            }
            let mut outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
}
