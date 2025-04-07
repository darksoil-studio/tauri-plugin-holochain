use std::sync::{Arc, Mutex};

use lair_keystore::dependencies::{hc_seed_bundle::SharedLockedArray, sodoken::LockedArray};

/// Convert a `Vec<u8>` to a `SharedLockedArray` as needed for passing a password into lair keystore.
pub fn vec_to_locked(pass_tmp: Vec<u8>) -> SharedLockedArray {
    Arc::new(Mutex::new(LockedArray::from( pass_tmp)))
}
