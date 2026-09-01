// SPDX-License-Identifier: GPL-3.0-or-later
pub mod boundary;
pub mod buffer;
pub mod matcher;

#[cfg(test)]
mod tests;

pub use boundary::{is_terminator, is_word_char, BoundaryMode};
pub use buffer::InputBuffer;
pub use matcher::{Match, Matcher, Trigger};

const CONTEXT_CHARS: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Backspace,

    Reset,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expansion {
    pub snippet_id: String,

    pub trigger: String,

    pub terminator: char,
}

#[derive(Debug)]
pub struct Engine {
    buffer: InputBuffer,
    matcher: Matcher,
    mode: BoundaryMode,
    enabled: bool,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            buffer: InputBuffer::new(1),
            matcher: Matcher::default(),
            mode: BoundaryMode::Word,
            enabled: true,
        }
    }

    pub fn set_triggers(&mut self, triggers: Vec<Trigger>) {
        self.matcher = Matcher::new(triggers);
        self.buffer
            .set_capacity(self.matcher.max_chars() + CONTEXT_CHARS);
        self.buffer.clear();
    }

    pub fn set_mode(&mut self, mode: BoundaryMode) {
        self.mode = mode;
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.buffer.clear();
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn trigger_count(&self) -> usize {
        self.matcher.len()
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
    }

    pub fn on_key(&mut self, key: Key) -> Option<Expansion> {
        if !self.enabled {
            self.buffer.clear();
            return None;
        }

        match key {
            Key::Reset => {
                self.buffer.clear();
                None
            }
            Key::Backspace => {
                self.buffer.backspace();
                None
            }
            Key::Char(c) => {
                if is_terminator(c) {
                    if let Some(found) = self.matcher.match_end(&self.buffer.as_vec(), self.mode) {
                        self.buffer.clear();
                        return Some(Expansion {
                            snippet_id: found.snippet_id,
                            trigger: found.trigger,
                            terminator: c,
                        });
                    }
                }
                self.buffer.push(c);
                None
            }
        }
    }

    pub fn note_injected_terminator(&mut self, terminator: char) {
        self.buffer.clear();
        self.buffer.push(terminator);
    }

    #[cfg(test)]
    pub(crate) fn buffer_contents(&self) -> String {
        self.buffer.as_vec().into_iter().collect()
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}
