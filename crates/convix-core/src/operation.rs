//! Operations a converter offers, described as data so the UI can render them generically.

use crate::format::Format;
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
