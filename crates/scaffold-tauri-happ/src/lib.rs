use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Input};
use file_tree_utils::{dir_to_file_tree, map_file, FileTree, FileTreeError};
use handlebars::{no_escape, RenderErrorReason};
use include_dir::{include_dir, Dir};
use nix_scaffolding_utils::{add_flake_input_to_flake_file, NixScaffoldingUtilsError};
use npm_scaffolding_utils::{
    add_npm_dev_dependency_to_package, add_npm_script_to_package, choose_npm_package, get_npm_package_name, guess_or_choose_package_manager, NpmScaffoldingUtilsError, PackageManager
};
use rust_scaffolding_utils::add_member_to_workspace;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use templates_scaffolding_utils::{
    helpers::merge::register_merge, register_case_helpers, render_template_file_tree_and_merge_with_existing, TemplatesScaffoldingUtilsError
};
use thiserror::Error;
use convert_case::{Case, Casing};

static TEMPLATE: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/templates/end-user-happ");

#[derive(Error, Debug)]
pub enum ScaffoldEndUserHappError {
    #[error(transparent)]
    NpmScaffoldingUtilsError(#[from] NpmScaffoldingUtilsError),

    #[error(transparent)]
    RenderError(#[from] RenderErrorReason),

    #[error(transparent)]
    TemplatesScaffoldingUtilsError(#[from] TemplatesScaffoldingUtilsError),

    #[error(transparent)]
    HolochainScaffoldingUtilsError(#[from] holochain_scaffolding_utils::Error),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    RegexError(#[from] regex::Error),

    #[error(transparent)]
    NixScaffoldingUtilsError(#[from] NixScaffoldingUtilsError),

    #[error(transparent)]
    RustScaffoldingUtilsError(#[from] rust_scaffolding_utils::Error),

    #[error(transparent)]
    DialoguerError(#[from] dialoguer::Error),

    #[error(transparent)]
    FileTreeError(#[from] FileTreeError),

    #[error("JSON serialization error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("Invalid identifier: {0}")]
    InvalidIdentifierError(String),

    #[error("Malformed package.json {0}: {1}")]
    MalformedJsonError(PathBuf, String),
}

#[derive(Serialize, Deserialize, Debug)]
struct ScaffoldEndUserHappData {
    app_name: String,
    app_bundle_location_from_root: PathBuf,
    identifier: String,
    package_manager: PackageManager,
}

fn validate_identifier(identifier: &String) -> Result<(), ScaffoldEndUserHappError> {
    if identifier.contains("-") || identifier.contains("_") {
        Err(ScaffoldEndUserHappError::InvalidIdentifierError( String::from("The bundle identifier can only contain alphanumerical characters.")))
    } else if identifier.split(".").collect::<Vec<&str>>().len() != 3 {
        Err(ScaffoldEndUserHappError::InvalidIdentifierError(String::from("The bundle identifier must contain three segments split by points (eg. 'org.myorg.myapp').")))
    } else {
        Ok(())
    }
}

pub fn scaffold_tauri_happ(
    file_tree: FileTree,
    ui_package: Option<String>,
    bundle_identifier: Option<String>,
) -> Result<FileTree, ScaffoldEndUserHappError> {
    // - Detect npm package manager
    let package_manager = guess_or_choose_package_manager(&file_tree)?;

    // - Guess the name of the app -> from the happ.yaml file
    let (happ_manifest_path, happ_manifest) =
        holochain_scaffolding_utils::get_or_choose_app_manifest(&file_tree)?;
    let app_name = happ_manifest.app_name().to_string();

    // - Create the src-tauri directory structure
    let template_file_tree = dir_to_file_tree(&TEMPLATE)?;
    let mut h = handlebars::Handlebars::new();
    h.register_escape_fn(no_escape);
    let h = register_case_helpers(h);
    let h = register_merge(h);

    let identifier: String = match bundle_identifier {
        Some(i) => {
            validate_identifier(&i)?;
            i
        },
        None => Input::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Input the bundle identifier for your app (eg: org.myorg.{}): ", app_name.to_case(Case::Flat)))
            .validate_with(|input: &String| validate_identifier(input))
            .interact_text()?
    };

    let mut app_bundle_location_from_root = happ_manifest_path.clone();
    app_bundle_location_from_root.pop();
    app_bundle_location_from_root = app_bundle_location_from_root.join(format!("{app_name}.happ"));

    let mut file_tree = render_template_file_tree_and_merge_with_existing(
        file_tree,
        &h,
        &template_file_tree,
        &ScaffoldEndUserHappData {
            identifier,
            app_name: app_name.clone(),
            app_bundle_location_from_root,
            package_manager
        },
    )?;

    let workspace_cargo_toml_path = PathBuf::from("Cargo.toml");

    map_file(
       &mut file_tree,
       &workspace_cargo_toml_path.clone().as_path(),
       |cargo_toml_content| {
           add_member_to_workspace(&(workspace_cargo_toml_path.clone(), cargo_toml_content), String::from("src-tauri"))
       },
    )?;

    let ui_package = match ui_package {
        Some(ui_package) => ui_package,
        None => {
            let npm_package = choose_npm_package(&file_tree, &String::from("Which NPM package contains your UI?\n\nThis is needed so that the NPM scripts can start the UI and tauri can connect to it."))?;
            let name = get_npm_package_name(&npm_package)?;
            name
        },
    };

    // - In package.json
    // - Add "start", "network", "build:zomes"
    let root_package_json_path = PathBuf::from("package.json");
    map_file(
        &mut file_tree,
        root_package_json_path.clone().as_path(),
        |package_json_content| {
            let package_json_content = add_npm_dev_dependency_to_package(
                &(root_package_json_path.clone(), package_json_content),
                &String::from("@tauri-apps/cli"),
                &String::from("^2.0.0"),
            )?;
            let package_json_content = add_npm_dev_dependency_to_package(
                &(root_package_json_path.clone(), package_json_content),
                &String::from("concurrently"),
                &String::from("^8.2.2"),
            )?;
            let package_json_content = add_npm_dev_dependency_to_package(
                &(root_package_json_path.clone(), package_json_content),
                &String::from("concurrently-repeat"),
                &String::from("^0.0.1"),
            )?;
            let package_json_content = add_npm_dev_dependency_to_package(
                &(root_package_json_path.clone(), package_json_content),
                &String::from("new-port-cli"),
                &String::from("^1.0.0"),
            )?;
            let package_json_content = add_npm_script_to_package(
                &(root_package_json_path.clone(), package_json_content),
                &String::from("start"),
                &format!(
                    "AGENTS=2 {}",
                    package_manager.run_script_command("network".into(), None)
                ),
            )?;
            let package_json_content = add_npm_script_to_package(
                &(root_package_json_path.clone(), package_json_content),
                &String::from("network"),
                &format!("{} && concurrently -k \"UI_PORT=1420 {}\" \"{}\"", 
                    
                    package_manager.run_script_command(String::from("build:happ"), None),
                    package_manager.run_script_command(String::from("start"), Some(ui_package.clone())),
                    package_manager.run_script_command(String::from("launch"), None)
                ),
            )?;

            let package_json_content = add_npm_script_to_package(
                &(root_package_json_path.clone(), package_json_content),
                &String::from("network:android"),
                &format!("{} && concurrently -k \"UI_PORT=1420 {}\" \"{}\" \"{}\"",
                    package_manager.run_script_command(String::from("build:happ"), None),
                    package_manager.run_script_command(String::from("start"), Some(ui_package.clone())),
                    package_manager.run_script_command(String::from("tauri dev"), None),
                    package_manager.run_script_command(String::from("tauri android dev"), None),
                ),
            )?;

            let package_json_content = add_npm_script_to_package(
                &(root_package_json_path.clone(), package_json_content),
                &String::from("build:zomes"),
                &format!("CARGO_TARGET_DIR=target cargo build --release --target wasm32-unknown-unknown --workspace --exclude {app_name}-tauri"),
            )?;
            let package_json_content = add_npm_script_to_package(
                &(root_package_json_path.clone(), package_json_content),
                &String::from("launch"),
                &format!(
                    "concurrently-repeat \"{}\" $AGENTS",
                    package_manager.run_script_command("tauri dev --no-watch".into(), None)
                ),
            )?;
            add_npm_script_to_package(
                &(root_package_json_path.clone(), package_json_content),
                &String::from("tauri"),
                &String::from("tauri"),
            )
        },
    )?;

    // - In ui/package.json
    map_file(
        &mut file_tree,
        PathBuf::from("ui/package.json").as_path(),
        |ui_package_json| {
            add_npm_script_to_package(
                &(root_package_json_path.clone(), ui_package_json),
                &String::from("start"),
                &String::from("vite --clearScreen false"),
            )
    })?;

    // - In flake.nix
    map_file(
        &mut file_tree,
        PathBuf::from("flake.nix").as_path(),
        |flake_nix_content| {
            let flake_nix_content = 
                flake_nix_content.replace("            rust # For Rust development, with the WASM target included for zome builds","" );

            // - Add the `tauri-plugin-holochain` as input to the flake
            let flake_nix_content = add_flake_input_to_flake_file(
                flake_nix_content,
                String::from("tauri-plugin-holochain"),
                String::from("github:darksoil-studio/tauri-plugin-holochain/main-0.5"),
            )?;

            let scope_opener = String::from("devShells.default = pkgs.mkShell {");

            let (mut open, mut close) =
                get_scope_open_and_close_char_indexes(&flake_nix_content, &scope_opener)?;
            // Move the open character to the beginning of the line for the scope opener
            open -= scope_opener.len();
            while flake_nix_content.chars().nth(open).unwrap() == ' '
                || flake_nix_content.chars().nth(open).unwrap() == '\t'
            {
                open -= 1;
            }
            close += 2;

            // TODO: check if there is per system, and if there is, check if there are inputs' in there 

            // - Add an androidDev devshell by copying the default devShell, and adding the holochainTauriAndroidDev
            let android_dev_shell = flake_nix_content[open..close]
                .to_string()
                .clone()
                .replace("holonix.devShells.default", "holonix.devShells.def2ault")
                .replace("default", "androidDev")
                .replace(
                    "inputsFrom = [",
                    r#"inputsFrom = [
              inputs'.tauri-plugin-holochain.devShells.holochainTauriAndroidDev"#,
                )
                .replace("holonix.devShells.def2ault", "holonix.devShells.default");

            // - Add the holochainTauriDev to the default devShell
            let default_dev_shell = flake_nix_content[open..close].to_string().replace(
                "inputsFrom = [",
                r#"inputsFrom = [
              inputs'.tauri-plugin-holochain.devShells.holochainTauriDev"#,
            );

            let flake_nix_content = format!(
                "{}{}{}{}",
                &flake_nix_content[..open],
                default_dev_shell,
                android_dev_shell,
                &flake_nix_content[close..]
            );

            let result: Result<String, ScaffoldEndUserHappError> = Ok(flake_nix_content);
            result
        },
    )?;

    Ok(file_tree)
}

pub fn get_scope_open_and_close_char_indexes(
    text: &String,
    scope_opener: &String,
) -> Result<(usize, usize), RenderErrorReason> {
    let mut index = text
        .find(scope_opener.as_str())
        .ok_or(RenderErrorReason::Other(
            "Given scope opener not found in the given parameter".into(),
        ))?;

    index = index + scope_opener.len() - 1;
    let scope_opener_index = index.clone();
    let mut scope_count = 1;

    while scope_count > 0 {
        index += 1;
        match text.chars().nth(index) {
            Some('{') => {
                scope_count += 1;
            }
            Some('}') => {
                scope_count -= 1;
            }
            None => {
                return Err(RenderErrorReason::Other("Malformed scopes".into()));
            }
            _ => {}
        }
    }

    // let mut whitespace = true;

    // while whitespace {
    //     match text.chars().nth(index - 1) {
    //         Some(' ') => {
    //             index -= 1;
    //         }
    //         _ => {
    //             whitespace = false;
    //         }
    //     }
    // }

    Ok((scope_opener_index, index))
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use super::*;
    use build_fs_tree::{dir, file};
    use file_tree_utils::file_content;

    #[test]
    fn simple_case_test() {
        let repo: FileTree = dir! {
            "flake.nix" => file!(default_flake_nix()),
            "workdir2" => dir! {
                "happ.yaml" => file!(empty_happ_yaml("myhapp"))
            },
            "ui" => dir! {
                "vite.config.ts" => file!(default_vite_config()),
                "package.json" => file!(empty_package_json("package1"))
            },
            "Cargo.toml" => file!(workspace_cargo_toml()),
            "package.json" => file!(empty_package_json("root")),
            "package-lock.json" => file!(empty_package_json("root")),
        };

        let repo = scaffold_tauri_happ(repo, Some(String::from("package1")), Some(String::from("studio.darksoil.myapp"))).unwrap();

        assert_eq!(
            file_content(&repo, PathBuf::from("package.json").as_path()).unwrap(),
            r#"{
  "name": "root",
  "dependencies": {},
  "scripts": {
    "build:happ": "npm run build:zomes && hc app pack workdir2",
    "start": "AGENTS=2 npm run network",
    "network": "npm run build:happ && concurrently -k \"UI_PORT=1420 npm run -w package1 start\" \"npm run launch\"",
    "network:android": "npm run build:happ && concurrently -k \"UI_PORT=1420 npm run -w package1 start\" \"npm run tauri dev\" \"npm run tauri android dev\"",
    "build:zomes": "CARGO_TARGET_DIR=target cargo build --release --target wasm32-unknown-unknown --workspace --exclude myhapp-tauri",
    "launch": "concurrently-repeat \"npm run tauri dev --no-watch\" $AGENTS",
    "tauri": "tauri"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.0.0",
    "concurrently": "^8.2.2",
    "concurrently-repeat": "^0.0.1",
    "new-port-cli": "^1.0.0"
  }
}"#
        );

        assert_eq!(
            file_content(&repo, PathBuf::from("flake.nix").as_path()).unwrap(),
            r#"{
  description = "Template for Holochain app development";
  
  inputs = {
    tauri-plugin-holochain.url = "github:darksoil-studio/tauri-plugin-holochain/main-0.5";
    nixpkgs.follows = "holonix/nixpkgs";

    holonix.url = "github:holochain/holonix/main-0.5";
    scaffolding.url = "github:darksoil-studio/scaffolding/main-0.5";
  };

  outputs = inputs @ { ... }:
    inputs.holonix.inputs.flake-parts.lib.mkFlake { inherit inputs; }
    {
      systems = builtins.attrNames inputs.holonix.devShells;
      perSystem =
        { inputs'
        , config
        , pkgs
        , system
        , lib
        , self'
        , ...
        }: {
          devShells.default = pkgs.mkShell {
            inputsFrom = [
              inputs'.tauri-plugin-holochain.devShells.holochainTauriDev 
              inputs'.scaffolding.devShells.synchronized-pnpm
              inputs'.holonix.devShells.default
            ];
          };
          devShells.androidDev = pkgs.mkShell {
            inputsFrom = [
              inputs'.tauri-plugin-holochain.devShells.holochainTauriAndroidDev 
              inputs'.scaffolding.devShells.synchronized-pnpm
              inputs'.holonix.devShells.default
            ];
          };
        };
    };
}
"#
        );

        assert!(
            file_content(&repo, PathBuf::from("src-tauri/src/lib.rs").as_path()).unwrap().contains("../../workdir2/myhapp.happ"),
        );

        assert_eq!(
            file_content(&repo, PathBuf::from("ui/vite.config.ts").as_path()).unwrap(),
            r#"import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// https://vitejs.dev/config/
export default defineConfig({
  server: {
    port: 1420,
    strictPort: true,
    host: "0.0.0.0",
    hmr: process.env.TAURI_DEV_HOST
      ? {
          protocol: "ws",
          host: process.env.TAURI_DEV_HOST,
          port: 1430,
        }
      : undefined,
  },
  plugins: [svelte()],
});
"#);

        assert_eq!(
            file_content(&repo, PathBuf::from("Cargo.toml").as_path()).unwrap(),
        r#"[patch.crates-io.wasmer-types]
branch = "fix-x86"
git = "https://github.com/guillemcordoba/wasmer"

[patch.crates-io.wasmer-vm]
branch = "fix-x86"
git = "https://github.com/guillemcordoba/wasmer"

[profile.dev]
opt-level = "z"

[profile.release]
opt-level = "z"

[workspace]
members = ["dnas/*/zomes/coordinator/*", "dnas/*/zomes/integrity/*", "src-tauri"]
resolver = "2"

[workspace.dependencies]
hdi = "0.5.0"
hdk = "0.4.0"
serde = "1.0"

[workspace.dependencies.posts]
path = "dnas/forum/zomes/coordinator/posts"

[workspace.dependencies.posts_integrity]
path = "dnas/forum/zomes/integrity/posts"
"#)
            }

            fn workspace_cargo_toml() -> String {
                String::from(r#"[profile.dev]
opt-level = "z"

[profile.release]
opt-level = "z"

[workspace]
resolver = "2"
members = ["dnas/*/zomes/coordinator/*", "dnas/*/zomes/integrity/*"]

[workspace.dependencies]
hdi = "0.5.0"
hdk = "0.4.0"
serde = "1.0"

[workspace.dependencies.posts]
path = "dnas/forum/zomes/coordinator/posts"

[workspace.dependencies.posts_integrity]
path = "dnas/forum/zomes/integrity/posts"
"#)
    }

    fn empty_package_json(package_name: &str) -> String {
        format!(
            r#"{{
  "name": "{package_name}",
  "dependencies": {{}},
  "scripts": {{
    "build:happ": "npm run build:zomes && hc app pack workdir2"
  }}
}}
"#
        )
    }

    fn default_vite_config() -> String {
        String::from(
            r#"import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [svelte()],
});
"#)
        }

    fn empty_happ_yaml(happ_name: &str) -> String {
        format!(
            r#"
---
manifest_version: "1"
name: {happ_name}
description: ~
roles:
  - name: forum
    provisioning:
      strategy: create
      deferred: false
    dna:
      bundled: "../dnas/forum/workdir/forum.dna"
      modifiers:
        network_seed: ~
        properties: ~
      installed_hash: ~
      clone_limit: 0
"#
        )
    }

    fn default_flake_nix() -> String {
        String::from(
            r#"{
  description = "Template for Holochain app development";
  
  inputs = {
    nixpkgs.follows = "holonix/nixpkgs";

    holonix.url = "github:holochain/holonix/main-0.5";
    scaffolding.url = "github:darksoil-studio/scaffolding/main-0.5";
  };

  outputs = inputs @ { ... }:
    inputs.holonix.inputs.flake-parts.lib.mkFlake { inherit inputs; }
    {
      systems = builtins.attrNames inputs.holonix.devShells;
      perSystem =
        { inputs'
        , config
        , pkgs
        , system
        , lib
        , self'
        , ...
        }: {
          devShells.default = pkgs.mkShell {
            inputsFrom = [ 
              inputs'.scaffolding.devShells.synchronized-pnpm
              inputs'.holonix.devShells.default
            ];
          };
        };
    };
}
"#,
        )
    }
}
