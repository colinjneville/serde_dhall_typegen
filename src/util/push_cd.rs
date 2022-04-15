use std::{env, io, path};

/// Sets the current directory for the object's scope
pub struct PushCd {
    prev_current_dir: path::PathBuf,
}

impl PushCd {
    pub fn new<P: AsRef<path::Path>>(new_current_dir: P) -> Result<Self, String> {
        let prev_current_dir = match env::current_dir() {
            Ok(pb) => pb,
            Err(e) => return Err(format!("Failed to get current directory: '{}'", e)),
        };
        env::set_current_dir(new_current_dir).map_err(Self::setter_error_message)?;
        Ok(Self { prev_current_dir })
    }

    fn setter_error_message(e: io::Error) -> String {
        format!("Failed to set current directory: {}", e)
    }
}

impl Drop for PushCd {
    fn drop(&mut self) {
        env::set_current_dir(self.prev_current_dir.clone()).map_err(Self::setter_error_message).unwrap();
    }
}