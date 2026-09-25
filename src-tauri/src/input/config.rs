// SPDX-License-Identifier: GPL-3.0-or-later
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectionMode {
    Auto,
    Paste,
    Type,
}

impl InjectionMode {
    pub fn parse(value: &str) -> Self {
        match value {
            "paste" => InjectionMode::Paste,
            "type" => InjectionMode::Type,
            _ => InjectionMode::Auto,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardMode {
    Paste,
    Type,
}

impl ClipboardMode {
    pub fn parse(value: &str) -> Self {
        match value {
            "paste" => ClipboardMode::Paste,
            _ => ClipboardMode::Type,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypingSpeed {
    Fast,
    Balanced,
    Careful,
}

impl TypingSpeed {
    pub fn parse(value: &str) -> Self {
        match value {
            "fast" => TypingSpeed::Fast,
            "careful" => TypingSpeed::Careful,
            _ => TypingSpeed::Balanced,
        }
    }

    pub fn events_per_second(self) -> u32 {
        match self {
            TypingSpeed::Fast => 500,
            TypingSpeed::Balanced => 300,
            TypingSpeed::Careful => 120,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub preserve_terminator: bool,
    pub restore_clipboard: bool,
    pub injection: InjectionMode,
    pub typing: TypingSpeed,
    pub clipboard: ClipboardMode,

    pub attachment_settle_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            preserve_terminator: true,
            restore_clipboard: true,
            injection: InjectionMode::Auto,
            typing: TypingSpeed::Balanced,
            clipboard: ClipboardMode::Type,
            attachment_settle_ms: 500,
        }
    }
}

