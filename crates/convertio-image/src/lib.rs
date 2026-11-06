//! Image conversions, compression and resizing — built in, no download needed.

use convertio_core::{
    Converter, Error, Format, JobContext, OperationCategory, OperationDescriptor, OptionSpec,
    Result, operation::Choice,
};
use std::path::PathBuf;

pub struct ImageConverter;

/// Formats we can read.
const INPUTS: &[Format] = &[
    Format::Png,
    Format::Jpeg,
    Format::Webp,
    Format::Avif,
    Format::Gif,
    Format::Bmp,
    Format::Tiff,
    Format::Ico,
];

/// Formats we can write.
const OUTPUTS: &[Format] = &[
    Format::Png,
    Format::Jpeg,
    Format::Webp,
    Format::Avif,
    Format::Gif,
    Format::Bmp,
    Format::Tiff,
    Format::Ico,
];

fn quality(default: f64) -> OptionSpec {
    OptionSpec::Slider {
        id: "quality",
        label_key: "opt.quality",
        min: 1.0,
        max: 100.0,
        step: 1.0,
        default,
        unit: Some("%"),
    }
}

fn options_for_output(to: Format) -> Vec<OptionSpec> {
    match to {
        Format::Jpeg => vec![quality(90.0)],
        Format::Webp => vec![
            OptionSpec::Toggle { id: "lossless", label_key: "opt.lossless", default: false },
            quality(90.0),
        ],
        Format::Avif => vec![quality(75.0)],
        Format::Png => vec![OptionSpec::Toggle {
            id: "optimize",
            label_key: "opt.optimizeLossless",
            default: true,
        }],
        Format::Ico => vec![OptionSpec::Select {
            id: "size",
            label_key: "opt.iconSize",
            choices: vec![
                Choice::literal("256", "256 × 256"),
                Choice::literal("128", "128 × 128"),
                Choice::literal("64", "64 × 64"),
                Choice::literal("32", "32 × 32"),
            ],
            default: "256",
        }],
        _ => Vec::new(),
    }
}

impl Converter for ImageConverter {
    fn id(&self) -> &'static str {
        "image"
    }

    fn operations_for(&self, input: Format) -> Vec<OperationDescriptor> {
        if !INPUTS.contains(&input) {
            return Vec::new();
        }

        let mut ops: Vec<_> = OUTPUTS
            .iter()
            .filter(|&&to| to != input)
            .map(|&to| OperationDescriptor::convert(self.id(), to).with_options(options_for_output(to)))
            .collect();

        ops.push(OperationDescriptor {
            id: "image.compress.lossless".into(),
            converter: self.id(),
            category: OperationCategory::Compress,
            output: Some(input),
            label_key: "op.compressLossless",
            multi_input: false,
            requires: &[],
            options: Vec::new(),
        });
        ops.push(OperationDescriptor {
            id: "image.compress.smaller".into(),
            converter: self.id(),
            category: OperationCategory::Compress,
            output: Some(input),
            label_key: "op.compressSmaller",
            multi_input: false,
            requires: &[],
            options: vec![quality(75.0)],
        });
        ops.push(OperationDescriptor {
            id: "image.resize".into(),
            converter: self.id(),
            category: OperationCategory::Transform,
            output: Some(input),
            label_key: "op.resize",
            multi_input: false,
            requires: &[],
            options: vec![
                OptionSpec::Number { id: "width", label_key: "opt.width", min: Some(1.0), max: None, default: None, unit: Some("px") },
                OptionSpec::Number { id: "height", label_key: "opt.height", min: Some(1.0), max: None, default: None, unit: Some("px") },
                OptionSpec::Toggle { id: "keepRatio", label_key: "opt.keepRatio", default: true },
            ],
        });
        ops
    }

    fn run(&self, _ctx: &JobContext) -> Result<Vec<PathBuf>> {
        Err(Error::NotImplemented("image conversions (Phase 1)"))
    }
}
