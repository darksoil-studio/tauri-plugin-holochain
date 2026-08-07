---
"tauri-plugin-holochain": 'minor:feat'
"holochain_runtime": 'patch:enhance'
---

Add `init_deferred` + `HolochainExt::launch_holochain` to register the plugin without starting lair, the conductor, or any network traffic, and launch later (e.g. after a password gate). Adds `Error::AlreadyLaunched` / `Error::NotDeferred`; exiting before launch now shuts down cleanly. `HolochainRuntimeConfig` is now `Clone`.
