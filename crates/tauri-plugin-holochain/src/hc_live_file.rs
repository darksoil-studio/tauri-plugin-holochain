use std::{env::current_dir, fs};

pub fn create_hc_live_file(admin_port: u16) -> crate::Result<()> {
    let mut dir = current_dir()?;
    if dir.ends_with("src-tauri") {
        dir.pop();
    }
    let file = dir.join(format!(".hc_live_{admin_port}"));
    std::fs::write(&file, format!("{admin_port}"))?;

    Ok(())
}

pub fn delete_hc_live_file(admin_port: u16) -> crate::Result<()> {
    let mut dir = current_dir()?;
    if dir.ends_with("src-tauri") {
        dir.pop();
    }
    let file = dir.join(format!(".hc_live_{admin_port}"));
    fs::remove_file(file)?;

    Ok(())
}
