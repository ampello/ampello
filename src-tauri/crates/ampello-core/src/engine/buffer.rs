// SPDX-License-Identifier: GPL-3.0-or-later
use std::collections::VecDeque;

#[derive(Debug)]
pub struct InputBuffer {
    chars: VecDeque<char>,
    capacity: usize,
}

impl InputBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            chars: VecDeque::with_capacity(capacity.max(1)),
            capacity: capacity.max(1),
        }
    }

    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity.max(1);
        self.trim();
    }

    pub fn push(&mut self, c: char) {
        self.chars.push_back(c);
        self.trim();
    }

    pub fn backspace(&mut self) {
        self.chars.pop_back();
    }

    pub fn clear(&mut self) {
        self.chars.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    pub fn len(&self) -> usize {
        self.chars.len()
    }

    pub fn as_vec(&self) -> Vec<char> {
        self.chars.iter().copied().collect()
    }

    fn trim(&mut self) {
        while self.chars.len() > self.capacity {
            self.chars.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::InputBuffer;

    fn typed(buffer: &InputBuffer) -> String {
        buffer.as_vec().into_iter().collect()
    }

    #[test]
    fn keeps_only_the_most_recent_characters() {
        let mut buffer = InputBuffer::new(4);
        for c in "abcdef".chars() {
            buffer.push(c);
        }
        assert_eq!(typed(&buffer), "cdef");
    }

    #[test]
    fn backspace_removes_the_last_character() {
        let mut buffer = InputBuffer::new(8);
        for c in "hello".chars() {
            buffer.push(c);
        }
        buffer.backspace();
        buffer.backspace();
        assert_eq!(typed(&buffer), "hel");
    }

    #[test]
    fn backspace_on_empty_is_harmless() {
        let mut buffer = InputBuffer::new(8);
        buffer.backspace();
        assert!(buffer.is_empty());
    }

    #[test]
    fn growing_capacity_keeps_what_is_there() {
        let mut buffer = InputBuffer::new(2);
        for c in "abcd".chars() {
            buffer.push(c);
        }
        assert_eq!(typed(&buffer), "cd");
        buffer.set_capacity(6);
        buffer.push('e');
        assert_eq!(typed(&buffer), "cde");
    }

    #[test]
    fn multi_byte_characters_count_as_one() {
        let mut buffer = InputBuffer::new(3);
        for c in "🚀日本語".chars() {
            buffer.push(c);
        }
        assert_eq!(buffer.len(), 3);
        assert_eq!(typed(&buffer), "日本語");
    }
}
