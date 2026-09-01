//! Closed, data-first ratio Gauge component.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Gauge as RatatuiGauge, LineGauge, Widget};
use serde::{Deserialize, Serialize};

use crate::ChartValue;
use crate::components::{ComponentValidationError, validate_identifier, validate_text};

/// Renderer capability for the Gauge component.
pub const GAUGE_COMPONENT_CAPABILITY: &str = "gauge";

const MAX_LABEL_BYTES: usize = 4 * 1024;
const MAX_ACCESSIBILITY_BYTES: usize = 16 * 1024;

/// A ratio from zero through one with one label and optional activation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Gauge {
    pub id: String,
    pub ratio: ChartValue,
    pub label: String,
    pub accessibility_text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activate: Option<String>,
}

impl Gauge {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        ratio: f64,
        label: impl Into<String>,
        accessibility_text: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            ratio: ChartValue::new(ratio),
            label: label.into(),
            accessibility_text: accessibility_text.into(),
            activate: None,
        }
    }

    #[must_use]
    pub fn activate(mut self, action: impl Into<String>) -> Self {
        self.activate = Some(action.into());
        self
    }

    #[must_use]
    pub fn percentage_label(&self) -> String {
        format!(
            "{}  {}%",
            self.label,
            (self.ratio.value() * 100.0).round() as u64
        )
    }

    pub(crate) fn validate(&self, path: &str) -> Result<(), ComponentValidationError> {
        validate_identifier(&self.id, &format!("{path}.id"))?;
        if !self.ratio.value().is_finite() || !(0.0..=1.0).contains(&self.ratio.value()) {
            return Err(ComponentValidationError::new(
                format!("{path}.ratio"),
                "must be finite and between 0 and 1 inclusive",
            ));
        }
        validate_single_line(&self.label, &format!("{path}.label"))?;
        validate_text(
            &self.accessibility_text,
            MAX_ACCESSIBILITY_BYTES,
            &format!("{path}.accessibilityText"),
        )?;
        if self.accessibility_text.trim().is_empty() {
            return Err(ComponentValidationError::new(
                format!("{path}.accessibilityText"),
                "must not be empty",
            ));
        }
        if let Some(activate) = &self.activate {
            validate_identifier(activate, &format!("{path}.activate"))?;
        }
        Ok(())
    }

    pub(crate) fn replace_data_from(&mut self, replacement: Self) {
        self.ratio = replacement.ratio;
        self.label = replacement.label;
        self.accessibility_text = replacement.accessibility_text;
    }

    #[must_use]
    pub fn widget(&self) -> GaugeWidget<'_> {
        GaugeWidget {
            gauge: self,
            filled_style: Style::new().fg(Color::Cyan),
            unfilled_style: Style::new().fg(Color::DarkGray),
        }
    }
}

/// Ratatui Gauge for multi-line areas and LineGauge for one-row areas.
pub struct GaugeWidget<'a> {
    gauge: &'a Gauge,
    filled_style: Style,
    unfilled_style: Style,
}

impl GaugeWidget<'_> {
    #[must_use]
    pub const fn styles(mut self, filled: Style, unfilled: Style) -> Self {
        self.filled_style = filled;
        self.unfilled_style = unfilled;
        self
    }
}

impl Widget for GaugeWidget<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let ratio = self.gauge.ratio.value();
        let label = self.gauge.percentage_label();
        if area.height == 1 {
            LineGauge::default()
                .ratio(ratio)
                .label(label)
                .filled_style(self.filled_style)
                .unfilled_style(self.unfilled_style)
                .render(area, buffer);
        } else {
            RatatuiGauge::default()
                .ratio(ratio)
                .label(label)
                .gauge_style(self.filled_style)
                .style(self.unfilled_style)
                .use_unicode(true)
                .render(area, buffer);
        }
    }
}

fn validate_single_line(value: &str, path: &str) -> Result<(), ComponentValidationError> {
    validate_text(value, MAX_LABEL_BYTES, path)?;
    if value.contains('\n') || value.trim().is_empty() {
        return Err(ComponentValidationError::new(
            path,
            "must be a non-empty single line",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    #[test]
    fn ratio_and_label_are_owned_by_the_spec() {
        let gauge = Gauge::new("build", 0.625, "Build", "Build is 62.5 percent complete");
        assert!(gauge.validate("gauge").is_ok());
        assert_eq!(gauge.percentage_label(), "Build  63%");
        assert!(
            Gauge::new("bad", 1.1, "Bad", "Invalid")
                .validate("gauge")
                .is_err()
        );
    }

    #[test]
    fn uses_line_gauge_for_one_row_and_gauge_for_larger_areas() {
        let gauge = Gauge::new("build", 0.5, "Build", "Half complete");
        for height in [1, 3] {
            let mut terminal = Terminal::new(TestBackend::new(24, height)).unwrap();
            terminal
                .draw(|frame| frame.render_widget(gauge.widget(), frame.area()))
                .unwrap();
            assert!(
                terminal
                    .backend()
                    .buffer()
                    .content()
                    .iter()
                    .any(|cell| { !cell.symbol().trim().is_empty() })
            );
        }
    }
}
