//! Operations a converter offers, described as data so the UI can render them generically.

use crate::{Error, Result, format::Format};
use serde::Serialize;

/// Option values chosen by the user, keyed by [`OptionSpec`] id.
pub type OptionValues = serde_json::Map<String, serde_json::Value>;

/// How the UI groups operations in the action panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationCategory {
    /// Change format (PNG → JPEG).
    Convert,
    /// Reduce file size (lossless or lossy).
    Compress,
    /// Modify content, same format (resize, rotate, normalize…).
    Transform,
    /// Pull something out (audio from video, pages from PDF…).
    Extract,
    /// Several inputs → one output (merge PDFs, images → PDF).
    Combine,
}

/// One action the user can pick for a file.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationDescriptor {
    /// Globally unique, stable id: `"<converter>.<action>[.<format>]"`, e.g. `"image.convert.png"`.
    /// Stable because presets and the Explorer context menu reference it.
    pub id: String,
    /// Id of the [`crate::Converter`] that runs it.
    pub converter: &'static str,
    pub category: OperationCategory,
    /// Output format, when the operation produces a known one.
    pub output: Option<Format>,
    /// Appended to the file name, for results in the same format as the source
    /// (`photo.png` → `photo-min.png`). `None` for plain conversions.
    pub output_suffix: Option<&'static str>,
    /// i18n key for the label. For `Convert`, the UI can also build "Convertir en {output}".
    pub label_key: &'static str,
    /// Accepts several files at once and combines them (merge…).
    pub multi_input: bool,
    /// External modules needed (`"ffmpeg"`, `"pdfium"`). Empty = built-in.
    pub requires: &'static [&'static str],
    pub options: Vec<OptionSpec>,
}

impl OperationDescriptor {
    /// Shorthand for the most common case: a plain format conversion.
    pub fn convert(converter: &'static str, to: Format) -> Self {
        Self {
            id: format!("{converter}.convert.{}", to.extension()),
            converter,
            category: OperationCategory::Convert,
            output: Some(to),
            output_suffix: None,
            label_key: "op.convert",
            multi_input: false,
            requires: &[],
            options: Vec::new(),
        }
    }

    pub fn with_options(mut self, options: Vec<OptionSpec>) -> Self {
        self.options = options;
        self
    }

    pub fn requiring(mut self, modules: &'static [&'static str]) -> Self {
        self.requires = modules;
        self
    }

    /// Keep only valid values for the declared options, before a converter sees them.
    ///
    /// Values come from the UI but also from `options.json` (remembered options, editable
    /// by hand) and may be stale after an update. Unknown ids, wrong types and choices
    /// that no longer exist are dropped (the converter then uses its default); sliders
    /// are clamped. A number typed by the user outside its range is an error.
    pub fn sanitize_options(&self, values: &OptionValues) -> Result<OptionValues> {
        use serde_json::Value;
        let mut out = OptionValues::new();
        for spec in &self.options {
            let Some(v) = values.get(spec.id()) else { continue };
            let kept = match spec {
                OptionSpec::Slider { min, max, .. } => v.as_f64().map(|n| Value::from(n.clamp(*min, *max))),
                OptionSpec::Toggle { .. } => v.as_bool().map(Value::from),
                OptionSpec::Select { choices, .. } => v
                    .as_str()
                    .filter(|s| choices.iter().any(|c| c.value == *s))
                    .map(Value::from),
                OptionSpec::Text { .. } => v.as_str().map(|s| {
                    Value::from(s.chars().filter(|c| !c.is_control()).take(1000).collect::<String>())
                }),
                OptionSpec::Number { id, min, max, .. } => match v.as_f64() {
                    Some(n) if !n.is_finite()
                        || min.is_some_and(|m| n < m)
                        || max.is_some_and(|m| n > m) =>
                    {
                        return Err(Error::InvalidOption {
                            name: (*id).to_owned(),
                            reason: match (min, max) {
                                (Some(a), Some(b)) => format!("entre {a} et {b}"),
                                (Some(a), None) => format!("au moins {a}"),
                                (None, Some(b)) => format!("au plus {b}"),
                                (None, None) => "nombre invalide".into(),
                            },
                        });
                    }
                    Some(n) => Some(Value::from(n)),
                    None => None, // empty field = default
                },
            };
            if let Some(kept) = kept {
                out.insert(spec.id().to_owned(), kept);
            }
        }
        Ok(out)
    }
}

impl OptionSpec {
    pub fn id(&self) -> &'static str {
        match self {
            OptionSpec::Slider { id, .. }
            | OptionSpec::Toggle { id, .. }
            | OptionSpec::Select { id, .. }
            | OptionSpec::Text { id, .. }
            | OptionSpec::Number { id, .. } => id,
        }
    }
}

/// A user-tunable parameter. The UI renders the matching widget.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum OptionSpec {
    #[serde(rename_all = "camelCase")]
    Slider {
        id: &'static str,
        label_key: &'static str,
        min: f64,
        max: f64,
        step: f64,
        default: f64,
        unit: Option<&'static str>,
    },
    #[serde(rename_all = "camelCase")]
    Toggle {
        id: &'static str,
        label_key: &'static str,
        default: bool,
    },
    #[serde(rename_all = "camelCase")]
    Select {
        id: &'static str,
        label_key: &'static str,
        choices: Vec<Choice>,
        default: &'static str,
    },
    /// Free text, e.g. a page list "1-3, 5". Empty = the default behaviour.
    #[serde(rename_all = "camelCase")]
    Text {
        id: &'static str,
        label_key: &'static str,
        placeholder_key: &'static str,
    },
    #[serde(rename_all = "camelCase")]
    Number {
        id: &'static str,
        label_key: &'static str,
        min: Option<f64>,
        max: Option<f64>,
        default: Option<f64>,
        unit: Option<&'static str>,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    pub value: &'static str,
    /// i18n key, or a literal label when `literal` is true (e.g. "320 kbps").
    pub label: &'static str,
    pub literal: bool,
}

impl Choice {
    pub fn literal(value: &'static str, label: &'static str) -> Self {
        Self { value, label, literal: true }
    }
    pub fn key(value: &'static str, label_key: &'static str) -> Self {
        Self { value, label: label_key, literal: false }
    }
}

/// Helpers to read typed values with a fallback.
pub trait OptionValuesExt {
    fn f64_or(&self, id: &str, default: f64) -> f64;
    fn bool_or(&self, id: &str, default: bool) -> bool;
    fn str_or<'a>(&'a self, id: &str, default: &'a str) -> &'a str;
}

impl OptionValuesExt for OptionValues {
    fn f64_or(&self, id: &str, default: f64) -> f64 {
        self.get(id).and_then(|v| v.as_f64()).unwrap_or(default)
    }
    fn bool_or(&self, id: &str, default: bool) -> bool {
        self.get(id).and_then(|v| v.as_bool()).unwrap_or(default)
    }
    fn str_or<'a>(&'a self, id: &str, default: &'a str) -> &'a str {
        self.get(id).and_then(|v| v.as_str()).unwrap_or(default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn op() -> OperationDescriptor {
        OperationDescriptor::convert("audio", Format::Mp3).with_options(vec![
            OptionSpec::Slider { id: "quality", label_key: "", min: 1.0, max: 100.0, step: 1.0, default: 90.0, unit: None },
            OptionSpec::Select { id: "loudness", label_key: "", choices: vec![Choice::key("off", ""), Choice::key("-14", "")], default: "off" },
            OptionSpec::Number { id: "width", label_key: "", min: Some(1.0), max: Some(16384.0), default: None, unit: None },
            OptionSpec::Toggle { id: "keep", label_key: "", default: true },
        ])
    }

    fn values(v: serde_json::Value) -> OptionValues {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn drops_injected_and_unknown_values() {
        let clean = op()
            .sanitize_options(&values(json!({
                "quality": 500, "loudness": "loudnorm,amovie=C\\secret", "keep": "yes", "evil": 1, "width": null
            })))
            .unwrap();
        assert_eq!(clean, values(json!({ "quality": 100.0 })));
    }

    #[test]
    fn keeps_valid_values() {
        let v = values(json!({ "quality": 80, "loudness": "-14", "width": 800, "keep": false }));
        let clean = op().sanitize_options(&v).unwrap();
        assert_eq!(clean["loudness"], "-14");
        assert_eq!(clean["width"], 800.0);
        assert_eq!(clean["keep"], false);
    }

    #[test]
    fn number_out_of_range_is_an_error() {
        let err = op().sanitize_options(&values(json!({ "width": 200000 }))).unwrap_err();
        assert!(matches!(err, Error::InvalidOption { .. }));
    }
}
