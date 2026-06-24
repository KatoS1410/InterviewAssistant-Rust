use crossbeam_channel::Sender;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotkeyAction {
    LoopbackPress,
    LoopbackRelease,
    MicPress,
    MicRelease,
}

pub struct HotkeyService {
    _manager: GlobalHotKeyManager,
}

impl HotkeyService {
    pub fn install(tx: Sender<HotkeyAction>) -> anyhow::Result<Self> {
        let manager = GlobalHotKeyManager::new()?;
        let left = HotKey::new(Some(Modifiers::empty()), Code::ArrowLeft);
        let right = HotKey::new(Some(Modifiers::empty()), Code::ArrowRight);
        let left_id = left.id();
        let right_id = right.id();
        manager.register(left)?;
        manager.register(right)?;

        std::thread::Builder::new()
            .name("hotkeys".into())
            .spawn(move || {
                let receiver = GlobalHotKeyEvent::receiver();
                loop {
                    if let Ok(event) = receiver.recv() {
                        let action = match event.state {
                            HotKeyState::Pressed if event.id == left_id => {
                                Some(HotkeyAction::LoopbackPress)
                            }
                            HotKeyState::Released if event.id == left_id => {
                                Some(HotkeyAction::LoopbackRelease)
                            }
                            HotKeyState::Pressed if event.id == right_id => {
                                Some(HotkeyAction::MicPress)
                            }
                            HotKeyState::Released if event.id == right_id => {
                                Some(HotkeyAction::MicRelease)
                            }
                            _ => None,
                        };
                        if let Some(a) = action {
                            let _ = tx.send(a);
                        }
                    }
                }
            })
            .ok();

        Ok(Self { _manager: manager })
    }
}
