// Блокировка, чтобы нельзя было запустить второй экземпляр приложения.
// Создаёт lock-файл во временной папке и эксклюзивно его блокирует.

use std::fs::{File, OpenOptions};
use std::io::Write;

// Имя lock-файла.
const LOCK_NAME: &str = "interview_assistant_single_instance.lock";

// Держит lock-файл открытым, пока приложение живо.
// При дропе разблокирует и удаляет файл.
pub struct SingleInstanceGuard {
    _file: File,
    _path: std::path::PathBuf,
}

// Пытается захватить сингл-инстанс.
// Если lock-файл уже занят другим процессом — вернёт None.
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
    // Пишет PID текущего процесса в lock-файл (для отладки).
    fn write_pid(&mut self) {
        let _ = self._file.set_len(0);
        let _ = write!(self._file, "{}", std::process::id());
        let _ = self._file.flush();
    }
}

// При дропе снимаем блокировку и удаляем lock-файл.
impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        let _ = fs2::FileExt::unlock(&self._file);
        let _ = std::fs::remove_file(&self._path);
    }
}
