use std::fs::{File, OpenOptions};
use std::io::Write;

const LOCK_NAME: &str = "interview_assistant_single_instance.lock";

pub struct SingleInstanceGuard {
    _file: File,
    _path: std::path::PathBuf,
}

pub fn acquire_single_instance() -> Option<SingleInstanceGuard> {
    let path = std::env::temp_dir().join(LOCK_NAME);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .ok()?;

    fs2::FileExt::try_lock_exclusive(&file).ok()?;
    let mut guard = SingleInstanceGuard { _file: file, _path: path };
    guard.write_pid();
    Some(guard)
}

impl SingleInstanceGuard {
    fn write_pid(&mut self) {
        let _ = self._file.set_len(0);
        let _ = write!(self._file, "{}", std::process::id());
        let _ = self._file.flush();
    }
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        let _ = fs2::FileExt::unlock(&self._file);
        let _ = std::fs::remove_file(&self._path);
    }
}
