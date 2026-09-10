//! Per-game hotkey remapping for ReShade, the DLSS 5 add-on, and OptiScaler.
//!
//! | Binding | File | Section / key | Format |
//! |---|---|---|---|
//! | ReShade overlay | `ReShade.ini` | `[INPUT] KeyOverlay` | `vk,ctrl,shift,alt` |
//! | NR toggle (F6) | `ReShade.ini` | `[RenoDX.DLSS5] NRToggleKey` | VK int (`0` = unbound) |
//! | NR screenshot (F5) | `ReShade.ini` | `[RenoDX.DLSS5] NRScreenshotKey` | VK int (`0` = unbound) |
//! | OptiScaler menu | `OptiScaler.ini` | `[Menu] ShortcutKey` | int / `0xNN` / `auto` |

use crate::installer::{self, OPTI_INI};
use crate::reshade_ini::Ini;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

/// Virtual-key defaults (Win32).
pub const VK_HOME: u8 = 0x24;
pub const VK_INSERT: u8 = 0x2D;
pub const VK_PRIOR: u8 = 0x21; // Page Up
pub const VK_NEXT: u8 = 0x22; // Page Down
pub const VK_F5: u8 = 0x74;
pub const VK_F6: u8 = 0x75;

pub const SECTION_INPUT: &str = "INPUT";
pub const KEY_OVERLAY: &str = "KeyOverlay";
pub const SECTION_RENODX: &str = "RenoDX.DLSS5";
pub const KEY_NR_TOGGLE: &str = "NRToggleKey";
pub const KEY_NR_SHOT: &str = "NRScreenshotKey";
pub const SECTION_OPTI_MENU: &str = "Menu";
pub const KEY_OPTI_SHORTCUT: &str = "ShortcutKey";

/// ReShade-style chord: virtual key + Ctrl / Shift / Alt flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyChord {
    pub vk: u8,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl KeyChord {
    pub fn new(vk: u8) -> Self {
        Self {
            vk,
            ctrl: false,
            shift: false,
            alt: false,
        }
    }

    pub fn unbound() -> Self {
        Self::new(0)
    }

    pub fn is_bound(self) -> bool {
        self.vk != 0
    }

    pub fn parse(raw: &str) -> Self {
        let parts: Vec<&str> = raw.split(',').map(str::trim).collect();
        let vk = parts
            .first()
            .and_then(|s| s.parse::<u8>().ok())
            .unwrap_or(0);
        let flag = |i: usize| parts.get(i).and_then(|s| s.parse::<u8>().ok()).unwrap_or(0) != 0;
        Self {
            vk,
            ctrl: flag(1),
            shift: flag(2),
            alt: flag(3),
        }
    }

    pub fn encode(self) -> String {
        format!(
            "{},{},{},{}",
            self.vk,
            u8::from(self.ctrl),
            u8::from(self.shift),
            u8::from(self.alt)
        )
    }

    pub fn label(self) -> String {
        if !self.is_bound() {
            return "None".to_owned();
        }
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl".to_owned());
        }
        if self.shift {
            parts.push("Shift".to_owned());
        }
        if self.alt {
            parts.push("Alt".to_owned());
        }
        parts.push(vk_name(self.vk));
        parts.join("+")
    }
}

/// Editable bindings for one game folder (and optional `host64\` consumer dir).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameHotkeys {
    pub reshade_overlay: KeyChord,
    pub nr_toggle: u8,
    pub nr_screenshot: u8,
    pub opti_menu: u8,
    /// `ReShade.ini` exists (or was just created by us).
    pub has_reshade: bool,
    /// OptiScaler.ini exists.
    pub has_opti: bool,
    /// Write NR keys into this path when set (32-bit: `host64\`, else game dir).
    pub consumer_dir: PathBuf,
    pub game_dir: PathBuf,
}

impl Default for GameHotkeys {
    fn default() -> Self {
        Self {
            reshade_overlay: KeyChord::new(VK_HOME),
            nr_toggle: VK_F6,
            nr_screenshot: VK_F5,
            opti_menu: VK_INSERT,
            has_reshade: false,
            has_opti: false,
            consumer_dir: PathBuf::new(),
            game_dir: PathBuf::new(),
        }
    }
}

impl GameHotkeys {
    pub fn defaults_for(game_dir: &Path, consumer_dir: &Path) -> Self {
        Self {
            reshade_overlay: KeyChord::new(VK_HOME),
            nr_toggle: VK_F6,
            nr_screenshot: VK_F5,
            opti_menu: VK_INSERT,
            has_reshade: game_dir.join("ReShade.ini").is_file()
                || consumer_dir.join("ReShade.ini").is_file(),
            has_opti: game_dir.join(OPTI_INI).is_file(),
            consumer_dir: consumer_dir.to_path_buf(),
            game_dir: game_dir.to_path_buf(),
        }
    }
}

pub fn load(game_dir: &Path, consumer_dir: &Path) -> Result<GameHotkeys> {
    let mut h = GameHotkeys::defaults_for(game_dir, consumer_dir);

    let overlay_ini_path = preferred_reshade_ini(game_dir, consumer_dir);
    let nr_ini_path = {
        let p = consumer_dir.join("ReShade.ini");
        if p.is_file() {
            Some(p)
        } else {
            overlay_ini_path.clone()
        }
    };
    if overlay_ini_path.is_some() || nr_ini_path.is_some() {
        h.has_reshade = true;
    }
    if let Some(p) = &overlay_ini_path {
        let ini = Ini::load(p);
        if let Some(raw) = ini.get(SECTION_INPUT, KEY_OVERLAY) {
            h.reshade_overlay = KeyChord::parse(raw);
        }
    }
    if let Some(p) = &nr_ini_path {
        let ini = Ini::load(p);
        // Missing keys → stock F5/F6; an explicit `0` unbinds.
        h.nr_toggle = match ini.get(SECTION_RENODX, KEY_NR_TOGGLE) {
            Some(raw) => parse_vk_value(raw).unwrap_or(0),
            None => VK_F6,
        };
        h.nr_screenshot = match ini.get(SECTION_RENODX, KEY_NR_SHOT) {
            Some(raw) => parse_vk_value(raw).unwrap_or(0),
            None => VK_F5,
        };
    }

    let opti = game_dir.join(OPTI_INI);
    if opti.is_file() {
        h.has_opti = true;
        let text = std::fs::read_to_string(&opti)
            .with_context(|| format!("read {}", opti.display()))?;
        h.opti_menu = parse_opti_shortcut(ini_get_section_key(&text, SECTION_OPTI_MENU, KEY_OPTI_SHORTCUT).as_deref());
    }

    Ok(h)
}

pub fn save(h: &GameHotkeys) -> Result<()> {
    if !h.has_reshade && !h.has_opti {
        bail!("no ReShade.ini or OptiScaler.ini in this game folder yet — Install first");
    }

    if h.has_reshade || h.game_dir.join("ReShade.ini").is_file() || h.consumer_dir.join("ReShade.ini").is_file()
    {
        // Overlay key: prefer the ReShade next to the game exe (what Home opens in-game).
        let overlay_path = if h.game_dir.join("ReShade.ini").is_file() {
            h.game_dir.join("ReShade.ini")
        } else {
            h.consumer_dir.join("ReShade.ini")
        };
        if overlay_path.is_file() {
            let mut ini = Ini::load(&overlay_path);
            ini.set(SECTION_INPUT, KEY_OVERLAY, h.reshade_overlay.encode());
            ini.save(&overlay_path)?;
        }

        // NR hotkeys: always the consumer ReShade.ini (host64 for 32-bit).
        let nr_path = h.consumer_dir.join("ReShade.ini");
        if nr_path.is_file() {
            let mut ini = Ini::load(&nr_path);
            ini.set(SECTION_RENODX, KEY_NR_TOGGLE, h.nr_toggle.to_string());
            ini.set(SECTION_RENODX, KEY_NR_SHOT, h.nr_screenshot.to_string());
            ini.save(&nr_path)?;
        } else if h.game_dir.join("ReShade.ini").is_file() {
            let p = h.game_dir.join("ReShade.ini");
            let mut ini = Ini::load(&p);
            ini.set(SECTION_RENODX, KEY_NR_TOGGLE, h.nr_toggle.to_string());
            ini.set(SECTION_RENODX, KEY_NR_SHOT, h.nr_screenshot.to_string());
            ini.save(&p)?;
        }
    }

    if h.has_opti {
        let opti = h.game_dir.join(OPTI_INI);
        let text = std::fs::read_to_string(&opti)
            .with_context(|| format!("read {}", opti.display()))?;
        let value = format!("0x{:02X}", h.opti_menu);
        if let Some(new) = installer::set_ini_key(&text, SECTION_OPTI_MENU, KEY_OPTI_SHORTCUT, &value)
        {
            std::fs::write(&opti, new).with_context(|| format!("write {}", opti.display()))?;
        }
    }

    Ok(())
}

fn preferred_reshade_ini(game_dir: &Path, consumer_dir: &Path) -> Option<PathBuf> {
    let a = game_dir.join("ReShade.ini");
    if a.is_file() {
        return Some(a);
    }
    let b = consumer_dir.join("ReShade.ini");
    b.is_file().then_some(b)
}

fn parse_vk_value(raw: &str) -> Option<u8> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    // Accept "117", "117,0,0,0", "0x75".
    let head = s.split(',').next()?.trim();
    if let Some(hex) = head.strip_prefix("0x").or_else(|| head.strip_prefix("0X")) {
        return u8::from_str_radix(hex, 16).ok();
    }
    head.parse().ok()
}

fn parse_opti_shortcut(raw: Option<&str>) -> u8 {
    match raw.map(str::trim) {
        None | Some("") | Some("auto") | Some("Auto") => VK_INSERT,
        Some(s) => parse_vk_value(s).unwrap_or(VK_INSERT),
    }
}

/// Read `key` inside `[section]` from a loose OptiScaler-style ini.
fn ini_get_section_key(text: &str, section: &str, key: &str) -> Option<String> {
    let header = format!("[{section}]");
    let mut in_section = false;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_section = t.eq_ignore_ascii_case(&header);
            continue;
        }
        if !in_section || t.is_empty() || t.starts_with(';') || t.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = t.split_once('=') {
            if k.trim().eq_ignore_ascii_case(key) {
                return Some(v.trim().to_owned());
            }
        }
    }
    None
}

pub fn vk_name(vk: u8) -> String {
    match vk {
        0 => "None".into(),
        0x08 => "Backspace".into(),
        0x09 => "Tab".into(),
        0x0D => "Enter".into(),
        0x10 => "Shift".into(),
        0x11 => "Ctrl".into(),
        0x12 => "Alt".into(),
        0x13 => "Pause".into(),
        0x14 => "CapsLock".into(),
        0x1B => "Esc".into(),
        0x20 => "Space".into(),
        0x21 => "PageUp".into(),
        0x22 => "PageDown".into(),
        0x23 => "End".into(),
        0x24 => "Home".into(),
        0x25 => "Left".into(),
        0x26 => "Up".into(),
        0x27 => "Right".into(),
        0x28 => "Down".into(),
        0x2C => "PrintScreen".into(),
        0x2D => "Insert".into(),
        0x2E => "Delete".into(),
        0x30..=0x39 => ((vk - 0x30) + b'0').to_string(),
        0x41..=0x5A => ((vk - 0x41) + b'A').to_string(),
        0x60..=0x69 => format!("Num{}", vk - 0x60),
        0x6A => "Num*".into(),
        0x6B => "Num+".into(),
        0x6D => "Num-".into(),
        0x6E => "Num.".into(),
        0x6F => "Num/".into(),
        0x70..=0x7B => format!("F{}", vk - 0x6F),
        0x90 => "NumLock".into(),
        0x91 => "ScrollLock".into(),
        0xBA => ";".into(),
        0xBB => "=".into(),
        0xBC => ",".into(),
        0xBD => "-".into(),
        0xBE => ".".into(),
        0xBF => "/".into(),
        0xC0 => "`".into(),
        0xDB => "[".into(),
        0xDC => "\\".into(),
        0xDD => "]".into(),
        0xDE => "'".into(),
        _ => format!("VK_{vk:02X}"),
    }
}

/// Map an egui key to a Win32 virtual-key code.
pub fn egui_to_vk(key: eframe::egui::Key) -> Option<u8> {
    use eframe::egui::Key;
    Some(match key {
        Key::Escape => 0x1B,
        Key::Tab => 0x09,
        Key::Backspace => 0x08,
        Key::Enter => 0x0D,
        Key::Space => 0x20,
        Key::Insert => VK_INSERT,
        Key::Delete => 0x2E,
        Key::Home => VK_HOME,
        Key::End => 0x23,
        Key::PageUp => VK_PRIOR,
        Key::PageDown => VK_NEXT,
        Key::ArrowLeft => 0x25,
        Key::ArrowUp => 0x26,
        Key::ArrowRight => 0x27,
        Key::ArrowDown => 0x28,
        Key::F1 => 0x70,
        Key::F2 => 0x71,
        Key::F3 => 0x72,
        Key::F4 => 0x73,
        Key::F5 => VK_F5,
        Key::F6 => VK_F6,
        Key::F7 => 0x76,
        Key::F8 => 0x77,
        Key::F9 => 0x78,
        Key::F10 => 0x79,
        Key::F11 => 0x7A,
        Key::F12 => 0x7B,
        Key::A => 0x41,
        Key::B => 0x42,
        Key::C => 0x43,
        Key::D => 0x44,
        Key::E => 0x45,
        Key::F => 0x46,
        Key::G => 0x47,
        Key::H => 0x48,
        Key::I => 0x49,
        Key::J => 0x4A,
        Key::K => 0x4B,
        Key::L => 0x4C,
        Key::M => 0x4D,
        Key::N => 0x4E,
        Key::O => 0x4F,
        Key::P => 0x50,
        Key::Q => 0x51,
        Key::R => 0x52,
        Key::S => 0x53,
        Key::T => 0x54,
        Key::U => 0x55,
        Key::V => 0x56,
        Key::W => 0x57,
        Key::X => 0x58,
        Key::Y => 0x59,
        Key::Z => 0x5A,
        Key::Num0 => 0x30,
        Key::Num1 => 0x31,
        Key::Num2 => 0x32,
        Key::Num3 => 0x33,
        Key::Num4 => 0x34,
        Key::Num5 => 0x35,
        Key::Num6 => 0x36,
        Key::Num7 => 0x37,
        Key::Num8 => 0x38,
        Key::Num9 => 0x39,
        Key::Minus => 0xBD,
        Key::Equals => 0xBB,
        Key::OpenBracket => 0xDB,
        Key::CloseBracket => 0xDD,
        Key::Backslash => 0xDC,
        Key::Semicolon => 0xBA,
        Key::Quote => 0xDE,
        Key::Comma => 0xBC,
        Key::Period => 0xBE,
        Key::Slash => 0xBF,
        Key::Backtick => 0xC0,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn chord_roundtrip() {
        let c = KeyChord {
            vk: VK_HOME,
            ctrl: true,
            shift: false,
            alt: true,
        };
        assert_eq!(c.encode(), "36,1,0,1");
        assert_eq!(KeyChord::parse("36,1,0,1"), c);
        assert_eq!(c.label(), "Ctrl+Alt+Home");
    }

    #[test]
    fn parse_vk_variants() {
        assert_eq!(parse_vk_value("117"), Some(117));
        assert_eq!(parse_vk_value("117,0,0,0"), Some(117));
        assert_eq!(parse_vk_value("0x75"), Some(0x75));
        assert_eq!(parse_vk_value("0"), Some(0));
    }

    #[test]
    fn load_save_reshade_and_nr() {
        let t = tempdir().unwrap();
        let dir = t.path();
        fs::write(
            dir.join("ReShade.ini"),
            "[INPUT]\nKeyOverlay=36,0,0,0\n[RenoDX.DLSS5]\nNRToggleKey=117\nNRScreenshotKey=116\n",
        )
        .unwrap();
        let h = load(dir, dir).unwrap();
        assert_eq!(h.reshade_overlay.vk, VK_HOME);
        assert_eq!(h.nr_toggle, VK_F6);
        assert_eq!(h.nr_screenshot, VK_F5);

        let mut h2 = h.clone();
        h2.reshade_overlay = KeyChord {
            vk: 0x70,
            ctrl: true,
            shift: false,
            alt: false,
        };
        h2.nr_toggle = 0x77; // F8
        h2.nr_screenshot = 0;
        save(&h2).unwrap();

        let again = load(dir, dir).unwrap();
        assert_eq!(again.reshade_overlay.encode(), "112,1,0,0");
        assert_eq!(again.nr_toggle, 0x77);
        assert_eq!(again.nr_screenshot, 0);
    }

    #[test]
    fn load_save_opti_shortcut() {
        let t = tempdir().unwrap();
        let dir = t.path();
        fs::write(
            dir.join(OPTI_INI),
            "[Menu]\nShortcutKey=auto\nFpsShortcutKey=0x21\n",
        )
        .unwrap();
        let h = load(dir, dir).unwrap();
        assert!(h.has_opti);
        assert_eq!(h.opti_menu, VK_INSERT);

        let mut h2 = h.clone();
        h2.opti_menu = VK_F6;
        save(&h2).unwrap();
        let text = fs::read_to_string(dir.join(OPTI_INI)).unwrap();
        assert!(text.contains("ShortcutKey=0x75"), "{text}");
        let again = load(dir, dir).unwrap();
        assert_eq!(again.opti_menu, VK_F6);
    }

    #[test]
    fn consumer_dir_nr_keys() {
        let t = tempdir().unwrap();
        let game = t.path();
        let host = game.join("host64");
        fs::create_dir(&host).unwrap();
        fs::write(game.join("ReShade.ini"), "[INPUT]\nKeyOverlay=36,0,0,0\n").unwrap();
        fs::write(
            host.join("ReShade.ini"),
            "[RenoDX.DLSS5]\nNRToggleKey=0\nNRScreenshotKey=0\n",
        )
        .unwrap();
        let h = load(game, &host).unwrap();
        assert_eq!(h.nr_toggle, 0);
        assert_eq!(h.reshade_overlay.vk, VK_HOME);

        let mut h2 = h.clone();
        h2.nr_toggle = VK_F6;
        save(&h2).unwrap();
        let host_ini = Ini::load(&host.join("ReShade.ini"));
        assert_eq!(host_ini.get(SECTION_RENODX, KEY_NR_TOGGLE), Some("117"));
        let game_ini = Ini::load(&game.join("ReShade.ini"));
        assert!(game_ini.get(SECTION_RENODX, KEY_NR_TOGGLE).is_none());
    }
}
