//! Atom: a count, `caption-sm` muted text with tabular figures and thousands separators.

use crate::Text;
use crate::tokens::text;
use gpui_kit::ElementId;

impl Text {
    /// `value` as a count an enclosing accessible name reads.
    pub fn count(value: usize) -> Self {
        Self::new(format_count(value)).counted()
    }

    /// `value` as a count with a label of its own, `id`.
    pub fn labeled_count(id: impl Into<ElementId>, value: usize) -> Self {
        Self::labeled(id, format_count(value)).counted()
    }

    fn counted(self) -> Self {
        self.level(text::CAPTION_SM).muted().tabular()
    }
}

/// `value` with a comma between each group of three digits.
pub fn format_count(value: usize) -> String {
    let digits = value.to_string();
    let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            formatted.push(',');
        }
        formatted.push(digit);
    }
    formatted
}

#[cfg(test)]
mod tests;
