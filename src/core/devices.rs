use cpal::traits::{DeviceTrait, HostTrait};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioDeviceInfo {
    pub index: usize,
    pub name: String,
    pub channels: u16,
    pub is_output: bool,
}

const LOOPBACK_KEYWORDS: &[&str] = &[
    "loopback",
    "stereo mix",
    "стерео микшер",
    "стереомикшер",
    "what u hear",
    "what you hear",
    "wave out",
    "mixout",
    "mix out",
    "внутреннее аудио",
    "output mix",
    "cable output",
    "virtual audio",
    "blackhole",
    "monitor",
];

pub fn list_input_devices() -> Vec<AudioDeviceInfo> {
    let host = cpal::default_host();
    let mut devices = Vec::new();
    let mut index = 0;

    if let Ok(inputs) = host.input_devices() {
        for device in inputs {
            if let Ok(name) = device.name() {
                let channels = device
                    .default_input_config()
                    .map(|c| c.channels())
                    .unwrap_or(1);
                devices.push(AudioDeviceInfo {
                    index,
                    name,
                    channels,
                    is_output: false,
                });
                index += 1;
            }
        }
    }

    #[cfg(windows)]
    {
        if let Ok(outputs) = host.output_devices() {
            for device in outputs {
                if let Ok(name) = device.name() {
                    let label = format!("{name} [loopback]");
                    let channels = device
                        .default_output_config()
                        .map(|c| c.channels())
                        .unwrap_or(2);
                    devices.push(AudioDeviceInfo {
                        index,
                        name: label,
                        channels,
                        is_output: true,
                    });
                    index += 1;
                }
            }
        }
    }

    devices
}

pub fn find_loopback_device() -> Option<AudioDeviceInfo> {
    list_input_devices()
        .into_iter()
        .find(|d| is_loopback_name(&d.name))
}

pub fn find_mic_device() -> Option<AudioDeviceInfo> {
    list_input_devices()
        .into_iter()
        .find(|d| is_mic_name(&d.name))
}

pub fn is_loopback_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    LOOPBACK_KEYWORDS.iter().any(|kw| lower.contains(kw))
}

pub fn is_mic_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    !is_loopback_name(&lower)
        && (lower.contains("mic")
            || lower.contains("microphone")
            || lower.contains("микрофон"))
}

pub fn resolve_device(name: &str) -> Option<AudioDeviceInfo> {
    let target = name.trim();
    if target.is_empty() {
        return None;
    }
    let devices = list_input_devices();
    devices
        .iter()
        .find(|d| d.name == target)
        .or_else(|| {
            devices
                .iter()
                .find(|d| d.name.to_lowercase().contains(&target.to_lowercase()))
        })
        .cloned()
}
