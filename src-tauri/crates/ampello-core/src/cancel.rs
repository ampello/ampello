// SPDX-License-Identifier: GPL-3.0-or-later
//! The key that stops an insertion part-way through.
//!
//! Escape is the default, but it is also the key applications use for
//! themselves - a browser stops loading a page or leaves full screen on it - so
//! it can be changed. Only keys that are not text and rarely mean anything on
//! their own are offered, and only as a single key: an insertion first releases
//! the modifiers the user is holding, so a combination could not be read back
//! reliably.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CancelKey {
    #[default]
    Escape,
    Pause,
    ScrollLock,
    /// F1 to F12.
    Function(u8),
}

impl CancelKey {
    /// Every key the settings offer, in the order they are listed.
    pub fn all() -> Vec<CancelKey> {
        let mut keys = vec![CancelKey::Escape, CancelKey::Pause, CancelKey::ScrollLock];
        keys.extend((1..=12).map(CancelKey::Function));
        keys
    }

    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        match value {
            "Escape" => Some(CancelKey::Escape),
            "Pause" => Some(CancelKey::Pause),
            "ScrollLock" => Some(CancelKey::ScrollLock),
            _ => {
                let number: u8 = value.strip_prefix('F')?.parse().ok()?;
                (1..=12)
                    .contains(&number)
                    .then_some(CancelKey::Function(number))
            }
        }
    }

    pub fn name(self) -> String {
        match self {
            CancelKey::Escape => "Escape".into(),
            CancelKey::Pause => "Pause".into(),
            CancelKey::ScrollLock => "ScrollLock".into(),
            CancelKey::Function(number) => format!("F{number}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CancelKey;

    #[test]
    fn every_offered_key_round_trips_through_its_name() {
        for key in CancelKey::all() {
            assert_eq!(CancelKey::parse(&key.name()), Some(key));
        }
    }

    #[test]
    fn anything_else_is_refused() {
        for bad in ["", "Enter", "A", "F0", "F13", "Ctrl+F1", "escape"] {
            assert_eq!(CancelKey::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn escape_is_the_default() {
        assert_eq!(CancelKey::default(), CancelKey::Escape);
    }
}
