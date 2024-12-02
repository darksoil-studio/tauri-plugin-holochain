use lair_keystore::dependencies::sodoken::{BufRead, BufWrite};

/// Convert a `Vec<u8>` to a `BufRead` as needed for passing a password into lair keystore.
pub fn vec_to_locked(mut pass_tmp: Vec<u8>) -> std::io::Result<BufRead> {
  match BufWrite::new_mem_locked(pass_tmp.len()) {
      Err(e) => {
          pass_tmp.fill(0);
          Err(e.into())
      }
      Ok(p) => {
          {
              let mut lock = p.write_lock();
              lock.copy_from_slice(&pass_tmp);
              pass_tmp.fill(0);
          }
          Ok(p.to_read())
      }
  }
}
