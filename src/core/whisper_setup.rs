use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::thread::{self, JoinHandle};

use crossbeam_channel::{Receiver, Sender};

use crate::core::helpers::whisper_dir;

/// События прогресса при автоскачивании whisper.
#[derive(Clone, Debug)]
pub enum SetupEvent {
    /// Текстовый статус (что происходит прямо сейчас).
    Status(String),
    /// Прогресс в процентах (0..=100) для текущего файла.
    Progress(u8),
    /// Готово: путь к модели.
    Done { model: PathBuf },
    /// Ошибка.
    Error(String),
}

/// Какую модель качаем.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelKind {
    Base,
    Small,
    Medium,
}

impl ModelKind {
    pub fn url(&self) -> &'static str {
        match self {
            ModelKind::Base => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
            ModelKind::Small => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
            ModelKind::Medium => {
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin"
            }
        }
    }

    pub fn file_name(&self) -> &'static str {
        match self {
            ModelKind::Base => "ggml-base.bin",
            ModelKind::Small => "ggml-small.bin",
            ModelKind::Medium => "ggml-medium.bin",
        }
    }
}

/// Имя бинарника whisper под текущую платформу.
/// На Windows в новых релизах whisper.cpp бинарник зовётся whisper-cli.exe.
pub fn whisper_exe_name() -> &'static str {
    if cfg!(windows) { "whisper-cli.exe" } else { "whisper-cli" }
}

/// Запускает установку whisper в отдельном потоке.
/// Возвращает ручку потока и приёмник событий.
pub fn spawn_setup(model: ModelKind) -> (JoinHandle<()>, Receiver<SetupEvent>) {
    let (tx, rx) = crossbeam_channel::unbounded::<SetupEvent>();
    let handle = thread::Builder::new()
        .name("whisper-setup".into())
        .spawn(move || {
            if let Err(err) = run_setup(&tx, model) {
                let _ = tx.send(SetupEvent::Error(err.to_string()));
            }
        })
        .expect("spawn whisper setup");
    (handle, rx)
}

fn run_setup(tx: &Sender<SetupEvent>, model: ModelKind) -> Result<(), anyhow::Error> {
    let dir = whisper_dir();
    fs::create_dir_all(&dir)?;

    // 1. Бинарник whisper.cpp.
    let _ = tx.send(SetupEvent::Status("Проверка whisper.cpp...".into()));
    let _ = ensure_whisper_exe(tx, &dir)?;

    // 2. Модель.
    let _ = tx.send(SetupEvent::Status(format!(
        "Проверка модели ({})...",
        model.file_name()
    )));
    let model_path = ensure_model(tx, &dir, model)?;

    let _ = tx.send(SetupEvent::Status("Готово".into()));
    let _ = tx.send(SetupEvent::Done { model: model_path });
    Ok(())
}

fn ensure_whisper_exe(tx: &Sender<SetupEvent>, dir: &Path) -> Result<PathBuf, anyhow::Error> {
    let exe_name = whisper_exe_name();
    let local = dir.join(exe_name);
    if local.is_file() {
        return Ok(local);
    }

    let _ = tx.send(SetupEvent::Status("Скачивание whisper.cpp...".into()));

    // Имя ассета под платформу.
    let asset_name = if cfg!(windows) {
        "whisper-bin-x64.zip".to_string()
    } else if cfg!(target_os = "macos") {
        "whisper-bin-universal.zip".to_string()
    } else {
        "whisper-bin-ubuntu-x64.tar.gz".to_string()
    };

    let asset_url = format!(
        "https://github.com/ggml-org/whisper.cpp/releases/latest/download/{asset_name}"
    );

    let archive_path = dir.join(&asset_name);
    download_with_progress(tx, &asset_url, &archive_path)?;

    let _ = tx.send(SetupEvent::Status("Распаковка whisper.cpp...".into()));
    if asset_name.ends_with(".zip") {
        extract_zip_all(&archive_path, dir)?;
    } else {
        extract_tar_gz_all(&archive_path, dir)?;
    }

    let _ = fs::remove_file(&archive_path);

    if !local.is_file() {
        return Err(anyhow::anyhow!(
            "whisper.cpp ({exe_name}) не найден после распаковки: {}",
            local.display()
        ));
    }

    // На unix выдаём права на исполнение.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(&local) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(&local, perms);
        }
    }

    Ok(local)
}

fn ensure_model(
    tx: &Sender<SetupEvent>,
    dir: &Path,
    model: ModelKind,
) -> Result<PathBuf, anyhow::Error> {
    let path = dir.join(model.file_name());

    // Модель уже есть — выходим.
    if path.is_file() {
        return Ok(path);
    }

    // Чистим старые модели перед скачкой новой.
    let _ = tx.send(SetupEvent::Status(
        "Удаление старых моделей...".into(),
    ));
    let all_models = ["ggml-base.bin", "ggml-small.bin", "ggml-medium.bin",
                      "ggml-base.en.bin", "ggml-small.en.bin", "ggml-medium.en.bin"];
    for m in &all_models {
        let old = dir.join(m);
        if old.is_file() && old != path {
            let _ = fs::remove_file(&old);
        }
    }

    let _ = tx.send(SetupEvent::Status(format!(
        "Скачивание модели, пожалуйста, подождите\n{}",
        model.file_name()
    )));
    download_with_progress(tx, model.url(), &path)?;

    // Обновляем путь в конфиге.
    let _ = tx.send(SetupEvent::Status("Модель загружена".into()));
    Ok(path)
}

fn download_with_progress(
    tx: &Sender<SetupEvent>,
    url: &str,
    dest: &Path,
) -> Result<(), anyhow::Error> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("interview-assistant")
        .timeout(std::time::Duration::from_secs(600))
        .build()?;
    let resp = client.get(url).send()?;
    if !resp.status().is_success() {
        return Err(anyhow::anyhow!(
            "HTTP {} при скачивании {}",
            resp.status(),
            url
        ));
    }
    let total = resp.content_length();

    let mut file = fs::File::create(dest)?;
    let mut reader = resp;
    let mut buf = [0u8; 64 * 1024];
    let mut read_total: u64 = 0;
    let mut last_pct: u8 = 0;

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        read_total += n as u64;
        if let Some(total) = total {
            let pct = ((read_total as f64 / total as f64) * 100.0) as u8;
            if pct != last_pct {
                last_pct = pct;
                let _ = tx.send(SetupEvent::Progress(pct));
            }
        }
    }
    file.flush()?;
    Ok(())
}

/// Распаковывает ВСЕ файлы из zip в dest_dir, выравнивая структуру
/// (вложенные папки типа Release/ убираются — всё кладётся прямо в dest_dir).
fn extract_zip_all(archive: &Path, dest_dir: &Path) -> Result<(), anyhow::Error> {
    let file = fs::File::open(archive)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let _name = entry.name().to_string();
        let outpath = match entry.enclosed_name() {
            Some(p) => dest_dir.join(p.file_name().unwrap_or_default()),
            None => continue,
        };

        // Папки пропускаем.
        if entry.is_dir() {
            continue;
        }

        let mut outfile = fs::File::create(&outpath)?;
        io::copy(&mut entry, &mut outfile)?;
    }
    Ok(())
}

/// Распаковывает tar.gz (для Linux).
fn extract_tar_gz_all(archive: &Path, dest_dir: &Path) -> Result<(), anyhow::Error> {
    use std::process::Command;
    fs::create_dir_all(dest_dir)?;
    let output = Command::new("tar")
        .arg("xzf")
        .arg(archive)
        .arg("-C")
        .arg(dest_dir)
        .arg("--strip-components=1")
        .output()?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "tar extract failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

/// Проверяет, есть ли whisper.dll и хотя бы одна модель.
pub fn is_ready() -> bool {
    let dir = whisper_dir();
    let dll = dir.join("whisper.dll");
    if !dll.is_file() {
        return false;
    }
    for m in ["ggml-base.bin", "ggml-small.bin", "ggml-medium.bin"] {
        if dir.join(m).is_file() {
            return true;
        }
    }
    false
}
