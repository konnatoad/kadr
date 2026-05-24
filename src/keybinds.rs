use egui::Key;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyAction {
    NextImage,
    PrevImage,
    ToggleZoom,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    PanUp,
    PanDown,
    PanLeft,
    PanRight,
    Fullscreen,
    ToggleThumbnails,
    RotateCW,
    RotateCCW,
    FlipHorizontal,
    FlipVertical,
    OpenFolder,
    OpenFile,
    CombineFolders,
    ToggleSlideshow,
    OpenSettings,
    DeleteFile,
    Quit,
}

impl KeyAction {
    pub fn label(&self) -> &'static str {
        match self {
            KeyAction::NextImage => "Next image",
            KeyAction::PrevImage => "Previous image",
            KeyAction::ToggleZoom => "Toggle zoom (fit / real size)",
            KeyAction::ZoomIn => "Zoom in",
            KeyAction::ZoomOut => "Zoom out",
            KeyAction::ZoomReset => "Reset zoom",
            KeyAction::PanUp => "Pan up (when zoomed)",
            KeyAction::PanDown => "Pan down (when zoomed)",
            KeyAction::PanLeft => "Pan left (when zoomed)",
            KeyAction::PanRight => "Pan right (when zoomed)",
            KeyAction::Fullscreen => "Toggle fullscreen",
            KeyAction::ToggleThumbnails => "Toggle thumbnail strip",
            KeyAction::RotateCW => "Rotate clockwise",
            KeyAction::RotateCCW => "Rotate counter-clockwise",
            KeyAction::FlipHorizontal => "Flip horizontal",
            KeyAction::FlipVertical => "Flip vertical",
            KeyAction::OpenFolder => "Open folder",
            KeyAction::OpenFile => "Open file",
            KeyAction::CombineFolders => "Combine folders",
            KeyAction::ToggleSlideshow => "Toggle slideshow",
            KeyAction::OpenSettings => "Open settings",
            KeyAction::DeleteFile => "Delete file",
            KeyAction::Quit => "Quit",
        }
    }

    pub fn all() -> Vec<KeyAction> {
        vec![
            KeyAction::NextImage,
            KeyAction::PrevImage,
            KeyAction::ToggleZoom,
            KeyAction::ZoomIn,
            KeyAction::ZoomOut,
            KeyAction::ZoomReset,
            KeyAction::PanUp,
            KeyAction::PanDown,
            KeyAction::PanLeft,
            KeyAction::PanRight,
            KeyAction::Fullscreen,
            KeyAction::ToggleThumbnails,
            KeyAction::RotateCW,
            KeyAction::RotateCCW,
            KeyAction::FlipHorizontal,
            KeyAction::FlipVertical,
            KeyAction::OpenFolder,
            KeyAction::OpenFile,
            KeyAction::CombineFolders,
            KeyAction::ToggleSlideshow,
            KeyAction::OpenSettings,
            KeyAction::DeleteFile,
            KeyAction::Quit,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBinding {
    pub key: SerializableKey,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl KeyBinding {
    pub fn simple(key: SerializableKey) -> Self {
        Self { key, ctrl: false, shift: false, alt: false }
    }

    pub fn with_ctrl(key: SerializableKey) -> Self {
        Self { key, ctrl: true, shift: false, alt: false }
    }

    pub fn matches(&self, input: &egui::InputState) -> bool {
        let key: Key = self.key.into();
        input.key_pressed(key)
            && input.modifiers.ctrl == self.ctrl
            && input.modifiers.shift == self.shift
            && input.modifiers.alt == self.alt
    }

    pub fn display(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl { parts.push("Ctrl".to_string()); }
        if self.shift { parts.push("Shift".to_string()); }
        if self.alt { parts.push("Alt".to_string()); }
        parts.push(format!("{:?}", Key::from(self.key)));
        parts.join("+")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SerializableKey {
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Space,
    Enter,
    Escape,
    Delete,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,
    Plus, Minus, Comma, Period,
    Home, End, PageUp, PageDown,
}

impl From<SerializableKey> for Key {
    fn from(k: SerializableKey) -> Key {
        match k {
            SerializableKey::ArrowLeft => Key::ArrowLeft,
            SerializableKey::ArrowRight => Key::ArrowRight,
            SerializableKey::ArrowUp => Key::ArrowUp,
            SerializableKey::ArrowDown => Key::ArrowDown,
            SerializableKey::Space => Key::Space,
            SerializableKey::Enter => Key::Enter,
            SerializableKey::Escape => Key::Escape,
            SerializableKey::Delete => Key::Delete,
            SerializableKey::F1 => Key::F1,
            SerializableKey::F2 => Key::F2,
            SerializableKey::F3 => Key::F3,
            SerializableKey::F4 => Key::F4,
            SerializableKey::F5 => Key::F5,
            SerializableKey::F6 => Key::F6,
            SerializableKey::F7 => Key::F7,
            SerializableKey::F8 => Key::F8,
            SerializableKey::F9 => Key::F9,
            SerializableKey::F10 => Key::F10,
            SerializableKey::F11 => Key::F11,
            SerializableKey::F12 => Key::F12,
            SerializableKey::A => Key::A,
            SerializableKey::B => Key::B,
            SerializableKey::C => Key::C,
            SerializableKey::D => Key::D,
            SerializableKey::E => Key::E,
            SerializableKey::F => Key::F,
            SerializableKey::G => Key::G,
            SerializableKey::H => Key::H,
            SerializableKey::I => Key::I,
            SerializableKey::J => Key::J,
            SerializableKey::K => Key::K,
            SerializableKey::L => Key::L,
            SerializableKey::M => Key::M,
            SerializableKey::N => Key::N,
            SerializableKey::O => Key::O,
            SerializableKey::P => Key::P,
            SerializableKey::Q => Key::Q,
            SerializableKey::R => Key::R,
            SerializableKey::S => Key::S,
            SerializableKey::T => Key::T,
            SerializableKey::U => Key::U,
            SerializableKey::V => Key::V,
            SerializableKey::W => Key::W,
            SerializableKey::X => Key::X,
            SerializableKey::Y => Key::Y,
            SerializableKey::Z => Key::Z,
            SerializableKey::Num0 => Key::Num0,
            SerializableKey::Num1 => Key::Num1,
            SerializableKey::Num2 => Key::Num2,
            SerializableKey::Num3 => Key::Num3,
            SerializableKey::Num4 => Key::Num4,
            SerializableKey::Num5 => Key::Num5,
            SerializableKey::Num6 => Key::Num6,
            SerializableKey::Num7 => Key::Num7,
            SerializableKey::Num8 => Key::Num8,
            SerializableKey::Num9 => Key::Num9,
            SerializableKey::Plus => Key::Plus,
            SerializableKey::Minus => Key::Minus,
            SerializableKey::Comma => Key::Comma,
            SerializableKey::Period => Key::Period,
            SerializableKey::Home => Key::Home,
            SerializableKey::End => Key::End,
            SerializableKey::PageUp => Key::PageUp,
            SerializableKey::PageDown => Key::PageDown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindings(pub HashMap<KeyAction, KeyBinding>);

impl Default for KeyBindings {
    fn default() -> Self {
        let mut map = HashMap::new();
        map.insert(KeyAction::NextImage, KeyBinding::simple(SerializableKey::ArrowRight));
        map.insert(KeyAction::PrevImage, KeyBinding::simple(SerializableKey::ArrowLeft));
        map.insert(KeyAction::ToggleZoom, KeyBinding::simple(SerializableKey::Space));
        map.insert(KeyAction::ZoomIn, KeyBinding::simple(SerializableKey::Plus));
        map.insert(KeyAction::ZoomOut, KeyBinding::simple(SerializableKey::Minus));
        map.insert(KeyAction::ZoomReset, KeyBinding::simple(SerializableKey::Num0));
        map.insert(KeyAction::PanUp, KeyBinding::simple(SerializableKey::ArrowUp));
        map.insert(KeyAction::PanDown, KeyBinding::simple(SerializableKey::ArrowDown));
        map.insert(KeyAction::PanLeft, KeyBinding::simple(SerializableKey::ArrowLeft));
        map.insert(KeyAction::PanRight, KeyBinding::simple(SerializableKey::ArrowRight));
        map.insert(KeyAction::Fullscreen, KeyBinding::simple(SerializableKey::F11));
        map.insert(KeyAction::ToggleThumbnails, KeyBinding::simple(SerializableKey::T));
        map.insert(KeyAction::RotateCW, KeyBinding::simple(SerializableKey::R));
        map.insert(KeyAction::RotateCCW, KeyBinding { key: SerializableKey::R, ctrl: false, shift: true, alt: false });
        map.insert(KeyAction::FlipHorizontal, KeyBinding::simple(SerializableKey::H));
        map.insert(KeyAction::FlipVertical, KeyBinding::simple(SerializableKey::V));
        map.insert(KeyAction::OpenFolder, KeyBinding::with_ctrl(SerializableKey::O));
        map.insert(KeyAction::OpenFile, KeyBinding { key: SerializableKey::O, ctrl: true, shift: true, alt: false });
        map.insert(KeyAction::CombineFolders, KeyBinding::with_ctrl(SerializableKey::E));
        map.insert(KeyAction::ToggleSlideshow, KeyBinding::simple(SerializableKey::S));
        map.insert(KeyAction::OpenSettings, KeyBinding::with_ctrl(SerializableKey::Comma));
        map.insert(KeyAction::DeleteFile, KeyBinding::simple(SerializableKey::Delete));
        map.insert(KeyAction::Quit, KeyBinding::with_ctrl(SerializableKey::Q));
        Self(map)
    }
}

impl KeyBindings {
    pub fn get(&self, action: &KeyAction) -> Option<&KeyBinding> {
        self.0.get(action)
    }

    pub fn is_action(&self, action: &KeyAction, input: &egui::InputState) -> bool {
        self.0.get(action).map(|b| b.matches(input)).unwrap_or(false)
    }
}
