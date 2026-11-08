use anyhow::{Context, Result};
use eframe::egui;
use global_hotkey::hotkey::HotKey;

// Register first so a conflicting new shortcut preserves the working binding.
pub fn replace(
    manager: &global_hotkey::GlobalHotKeyManager,
    registered: &mut Option<HotKey>,
    target: Option<HotKey>,
) -> Result<()> {
    if *registered == target {
        return Ok(());
    }
    if let Some(next) = target {
        manager.register(next)?;
    }
    if let Some(previous) = *registered {
        let _ = manager.unregister(previous);
    }
    *registered = target;
    Ok(())
}

pub fn parse(binding: &str) -> Result<HotKey> {
    binding
        .trim()
        .parse()
        .context("Choose a key or a shortcut such as F8 or Ctrl+Shift+V")
}

pub fn record(key: egui::Key, modifiers: egui::Modifiers) -> Option<String> {
    let name = match key {
        egui::Key::Num0 => "0".into(),
        egui::Key::Num1 => "1".into(),
        egui::Key::Num2 => "2".into(),
        egui::Key::Num3 => "3".into(),
        egui::Key::Num4 => "4".into(),
        egui::Key::Num5 => "5".into(),
        egui::Key::Num6 => "6".into(),
        egui::Key::Num7 => "7".into(),
        egui::Key::Num8 => "8".into(),
        egui::Key::Num9 => "9".into(),
        egui::Key::OpenBracket => "[".into(),
        egui::Key::CloseBracket => "]".into(),
        egui::Key::Backtick => "`".into(),
        _ => format!("{key:?}"),
    };
    let mut binding = String::new();
    if modifiers.ctrl {
        binding.push_str("Ctrl+");
    }
    if modifiers.alt {
        binding.push_str("Alt+");
    }
    if modifiers.shift {
        binding.push_str("Shift+");
    }
    if modifiers.mac_cmd {
        binding.push_str("Super+");
    }
    binding.push_str(&name);
    parse(&binding).ok().map(|_| binding)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn conflicting_shortcut_keeps_the_previous_binding_registered() {
        let manager = global_hotkey::GlobalHotKeyManager::new().unwrap();
        let old = parse("Ctrl+Alt+F10").unwrap();
        let conflict = parse("Ctrl+Alt+F11").unwrap();
        let mut registered = None;
        replace(&manager, &mut registered, Some(old)).unwrap();
        manager.register(conflict).unwrap();
        assert!(replace(&manager, &mut registered, Some(conflict)).is_err());
        assert_eq!(registered, Some(old));
        assert!(manager.register(old).is_err());
        manager.unregister(conflict).unwrap();
        replace(&manager, &mut registered, Some(conflict)).unwrap();
        manager.register(old).unwrap();
        manager.unregister(old).unwrap();
        replace(&manager, &mut registered, None).unwrap();
    }
    #[test]
    fn recorded_chords_match_the_native_hotkey_parser() {
        for (key, name) in [
            (egui::Key::V, "V"),
            (egui::Key::F8, "F8"),
            (egui::Key::Num4, "4"),
            (egui::Key::Space, "Space"),
            (egui::Key::ArrowLeft, "ArrowLeft"),
            (egui::Key::Backtick, "`"),
        ] {
            let mods = egui::Modifiers {
                ctrl: true,
                shift: true,
                ..Default::default()
            };
            let binding = record(key, mods).unwrap();
            assert_eq!(
                parse(&binding).unwrap(),
                parse(&format!("Ctrl+Shift+{name}")).unwrap()
            );
            assert_eq!(
                parse(&binding).unwrap(),
                parse(&parse(&binding).unwrap().to_string()).unwrap()
            );
        }
        for value in ["", "Ctrl", "Ctrl+", "F8+F9", "mouse4"] {
            assert!(parse(value).is_err());
        }
    }
}
