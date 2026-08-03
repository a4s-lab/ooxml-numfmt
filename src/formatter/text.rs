//! Text-value evaluation into unresolved render parts.

use crate::ast::FormatPart;
use crate::compile::SectionPlan;

use super::render::RenderPart;

/// Evaluate one selected text plan without resolving fill or skip directives.
pub(super) fn evaluate_text(plan: &SectionPlan, value: &str) -> Vec<RenderPart> {
    super::evaluate_operations(plan, |part| match part {
        FormatPart::TextPlaceholder => Some(value.to_string()),
        _ => None,
    })
}
