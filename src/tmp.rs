use std::{env, path::PathBuf, time::SystemTime};

/// Returns a path to a file in the OS's temp directory. The file isn't guaranteed to exist
/// already.
pub fn path() -> PathBuf {
    let since_epoch = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("after unix epoch");

    let mut path = env::temp_dir();
    path.push(format!("srs-cli_{}.txt", since_epoch.as_nanos()));

    path
}
