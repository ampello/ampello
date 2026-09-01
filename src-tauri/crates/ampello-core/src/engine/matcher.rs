// SPDX-License-Identifier: GPL-3.0-or-later
use std::collections::HashMap;

use super::boundary::{is_word_char, BoundaryMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trigger {
    pub snippet_id: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    pub snippet_id: String,
    pub trigger: String,

    pub trigger_chars: usize,
}

#[derive(Debug, Default)]
pub struct Matcher {
    by_text: HashMap<String, String>,

    lengths: Vec<usize>,
    max_chars: usize,
}

impl Matcher {
    pub fn new(triggers: Vec<Trigger>) -> Self {
        let mut by_text = HashMap::with_capacity(triggers.len());
        let mut lengths: Vec<usize> = Vec::new();

        for trigger in triggers {
            let chars = trigger.text.chars().count();
            if chars == 0 {
                continue;
            }
            if !lengths.contains(&chars) {
                lengths.push(chars);
            }
            by_text.insert(trigger.text, trigger.snippet_id);
        }

        lengths.sort_unstable_by(|a, b| b.cmp(a));
        let max_chars = lengths.first().copied().unwrap_or(0);

        Self {
            by_text,
            lengths,
            max_chars,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.by_text.is_empty()
    }

    pub fn len(&self) -> usize {
        self.by_text.len()
    }

    pub fn max_chars(&self) -> usize {
        self.max_chars
    }

    pub fn match_end(&self, buffer: &[char], mode: BoundaryMode) -> Option<Match> {
        if self.by_text.is_empty() || buffer.is_empty() {
            return None;
        }

        for &length in &self.lengths {
            if length > buffer.len() {
                continue;
            }
            let start = buffer.len() - length;

            if mode == BoundaryMode::Word && start > 0 && is_word_char(buffer[start - 1]) {
                continue;
            }

            let candidate: String = buffer[start..].iter().collect();
            if let Some(snippet_id) = self.by_text.get(&candidate) {
                return Some(Match {
                    snippet_id: snippet_id.clone(),
                    trigger: candidate,
                    trigger_chars: length,
                });
            }
        }
        None
    }
}
