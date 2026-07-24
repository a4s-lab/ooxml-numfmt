//! Structured formatting output.

use crate::ast::{Color, FormatPart};

/// An item in a formatted output stream.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum FormatOutput {
    /// Text to display.
    Text(String),
    /// Change the active color for subsequent paintable parts.
    Color(Color),
    /// Repeat this character to consume the available width.
    Fill(char),
    /// Reserve the rendered width of this character without painting it.
    Skip(char),
}

/// Convert structured formatting output to its plain-text approximation.
pub fn plain_text(output: &[FormatOutput]) -> String {
    let capacity = output
        .iter()
        .map(|part| match part {
            FormatOutput::Text(text) => text.len(),
            FormatOutput::Skip(_) => 1,
            FormatOutput::Color(_) | FormatOutput::Fill(_) => 0,
        })
        .sum();
    let mut text = String::with_capacity(capacity);

    for part in output {
        match part {
            FormatOutput::Text(value) => text.push_str(value),
            FormatOutput::Skip(_) => text.push(' '),
            FormatOutput::Color(_) | FormatOutput::Fill(_) => {}
        }
    }

    text
}

/// Converts nonempty text to a [`FormatOutput::Text`] part.
pub(crate) fn text_if_nonempty(text: impl Into<String>) -> Vec<FormatOutput> {
    let text = text.into();
    if text.is_empty() {
        Vec::new()
    } else {
        vec![FormatOutput::Text(text)]
    }
}

/// Converts a context-independent format part to structured output.
pub(crate) fn output_for_part(part: &FormatPart) -> Option<FormatOutput> {
    match part {
        FormatPart::Literal(text) | FormatPart::EscapedLiteral(text) => {
            Some(FormatOutput::Text(text.clone()))
        }
        FormatPart::Locale(locale) => locale.currency.clone().map(FormatOutput::Text),
        FormatPart::Percent => Some(FormatOutput::Text("%".to_string())),
        FormatPart::Fill(character) => Some(FormatOutput::Fill(*character)),
        FormatPart::Skip(character) => Some(FormatOutput::Skip(*character)),
        _ => None,
    }
}
