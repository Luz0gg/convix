//! Image conversions, compression and resizing — built in, no download needed.
//!
//! Operation ids (stable, referenced by presets and the Explorer menu):
//! - `image.convert.<ext>`      e.g. `image.convert.png`, `image.convert.jpg`
//! - `image.compress.lossless`  PNG only (oxipng)
//! - `image.compress.smaller`   PNG (palette), JPEG, WebP (lossy re-encode)
//! - `image.resize`

mod decode;
mod encode;
mod transform;

use convertio_core::{
    Converter, Error, Format, JobContext, OperationCategory, OperationDescriptor, OptionSpec,
    OptionValues, Result, format, operation::Choice, operation::OptionValuesExt, output,
};
use encode::{EncodeSettings, WRITABLE};
use std::path::PathBuf;

pub use encode::is_opaque;

pub struct ImageConverter;

/// Formats we can read. (AVIF and HEIC reading need native decoders: later.)
const READABLE: &[Format] = &[
    Format::Png,
    Format::Jpeg,
    Format::Webp,
    Format::Gif,
    Format::Bmp,
    Format::Tiff,
    Format::Ico,
    Format::Tga,
    Format::Svg,
];

pub fn can_read(f: Format) -> bool {
    READABLE.contains(&f)
}

// ---------- option specs ----------

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

fn svg_width() -> OptionSpec {
    OptionSpec::Number { id: "width", label_key: "opt.renderWidth", min: Some(1.0), max: Some(16384.0), default: None, unit: Some("px") }
}

fn convert_options(to: Format) -> Vec<OptionSpec> {
    match to {
        Format::Jpeg => vec![quality(90.0)],
        Format::Webp => vec![
            OptionSpec::Toggle { id: "lossless", label_key: "opt.lossless", default: false },
            quality(90.0),
        ],
        Format::Avif => vec![quality(75.0)],
        Format::Png => vec![OptionSpec::Toggle { id: "optimize", label_key: "opt.optimizeLossless", default: true }],
        Format::Ico => vec![OptionSpec::Select {
            id: "size",
            label_key: "opt.iconSize",
            choices: vec![
                Choice::key("multi", "opt.iconMulti"),
                Choice::literal("256", "256 × 256"),
                Choice::literal("128", "128 × 128"),
                Choice::literal("64", "64 × 64"),
                Choice::literal("32", "32 × 32"),
            ],
            default: "multi",
        }],
        _ => Vec::new(),
    }
}

fn settings_from(options: &OptionValues, to: Format) -> EncodeSettings {
    let mut s = EncodeSettings::for_format(to);
    s.quality = options.f64_or("quality", s.quality as f64) as f32;
    s.lossless = options.bool_or("lossless", false);
    if !options.bool_or("optimize", true) {
        s.png_optimize = None;
    }
    if let Ok(size) = options.str_or("size", "multi").parse::<u32>() {
        s.ico_sizes = vec![size];
    }
    s
}

fn descriptor(id: &str, category: OperationCategory, output: Format, label_key: &'static str, suffix: &'static str, options: Vec<OptionSpec>) -> OperationDescriptor {
    OperationDescriptor {
        id: id.to_owned(),
        converter: "image",
        category,
        output: Some(output),
        output_suffix: Some(suffix),
        label_key,
        multi_input: false,
        requires: &[],
        options,
    }
}

impl Converter for ImageConverter {
    fn id(&self) -> &'static str {
        "image"
    }

    fn operations_for(&self, input: Format) -> Vec<OperationDescriptor> {
        if !can_read(input) {
            return Vec::new();
        }

        let mut ops: Vec<_> = WRITABLE
            .iter()
            .filter(|&&to| to != input)
            .map(|&to| {
                let mut options = convert_options(to);
                if input == Format::Svg {
                    options.insert(0, svg_width());
                }
                OperationDescriptor::convert(self.id(), to).with_options(options)
            })
            .collect();

        if input == Format::Png {
            ops.push(descriptor(
                "image.compress.lossless",
                OperationCategory::Compress,
                input,
                "op.compressLossless",
                "-min",
                vec![OptionSpec::Select {
                    id: "effort",
                    label_key: "opt.effort",
                    choices: vec![
                        Choice::key("2", "opt.effortFast"),
                        Choice::key("4", "opt.effortBalanced"),
                        Choice::key("6", "opt.effortMax"),
                    ],
                    default: "4",
                }],
            ));
        }

        let smaller_options = match input {
            Format::Png => Some(vec![OptionSpec::Select {
                id: "colors",
                label_key: "opt.colors",
                choices: vec![
                    Choice::literal("256", "256"),
                    Choice::literal("128", "128"),
                    Choice::literal("64", "64"),
                    Choice::literal("32", "32"),
                ],
                default: "256",
            }]),
            Format::Jpeg | Format::Webp => Some(vec![quality(75.0)]),
            _ => None,
        };
        if let Some(options) = smaller_options {
            ops.push(descriptor("image.compress.smaller", OperationCategory::Compress, input, "op.compressSmaller", "-min", options));
        }

        if input != Format::Svg && input != Format::Ico {
            ops.push(descriptor(
                "image.resize",
                OperationCategory::Transform,
                input,
                "op.resize",
                "-resized",
                vec![
                    OptionSpec::Number { id: "width", label_key: "opt.width", min: Some(1.0), max: None, default: None, unit: Some("px") },
                    OptionSpec::Number { id: "height", label_key: "opt.height", min: Some(1.0), max: None, default: None, unit: Some("px") },
                    OptionSpec::Toggle { id: "keepRatio", label_key: "opt.keepRatio", default: true },
                ],
            ));
        }
        ops
    }

    fn run(&self, ctx: &JobContext) -> Result<Vec<PathBuf>> {
        let input = ctx.inputs.first().ok_or_else(|| Error::Failed("aucun fichier".into()))?;
        let in_fmt = format::detect(input)?.filter(|f| can_read(*f)).ok_or(Error::UnsupportedFormat)?;
        let opts = ctx.options;
        let unknown = || Error::UnknownOperation(ctx.operation_id.to_owned());
        let action = ctx.operation_id.strip_prefix("image.").ok_or_else(unknown)?;
        let num = |id: &str| opts.get(id).and_then(|v| v.as_f64()).filter(|v| *v > 0.0).map(|v| v.round() as u32);

        let bytes = if action == "compress.lossless" {
            if in_fmt != Format::Png {
                return Err(unknown());
            }
            let level = opts.str_or("effort", "4").parse().unwrap_or(4);
            let data = std::fs::read(input)?;
            ctx.cancel.check()?;
            encode::optimize_png(&data, level)?
        } else {
            let svg_w = if in_fmt == Format::Svg { num("width") } else { None };
            let decoded = decode::decode(input, in_fmt, svg_w)?;
            let icc = decoded.icc.as_deref();
            ctx.cancel.check()?;
            ctx.progress.progress(0.4);

            if let Some(ext) = action.strip_prefix("convert.") {
                let to = Format::from_extension(ext).filter(|f| WRITABLE.contains(f)).ok_or_else(unknown)?;
                encode::encode(&decoded.image, icc, to, &settings_from(opts, to))?
            } else if action == "compress.smaller" {
                let mut s = EncodeSettings::for_format(in_fmt);
                match in_fmt {
                    Format::Png => {
                        let colors = opts.str_or("colors", "256").parse().unwrap_or(256);
                        s.png_optimize = Some(3);
                        encode::encode(&transform::quantize(&decoded.image, colors), icc, Format::Png, &s)?
                    }
                    Format::Jpeg | Format::Webp => {
                        s.quality = opts.f64_or("quality", 75.0) as f32;
                        encode::encode(&decoded.image, icc, in_fmt, &s)?
                    }
                    _ => return Err(unknown()),
                }
            } else if action == "resize" {
                let resized = transform::resize(&decoded.image, num("width"), num("height"), opts.bool_or("keepRatio", true))?;
                encode::encode(&resized, icc, in_fmt, &EncodeSettings::for_format(in_fmt))?
            } else {
                return Err(unknown());
            }
        };

        ctx.cancel.check()?;
        output::write_atomic(&ctx.output, &bytes)?;
        ctx.progress.progress(1.0);
        Ok(vec![ctx.output.clone()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use convertio_core::{CancelToken, Registry, output::OutputPolicy};
    use image::{Rgba, RgbaImage};
    use std::path::Path;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("convertio-img-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 64×48 gradient with a transparent corner.
    fn sample_png(dir: &Path) -> PathBuf {
        let img = RgbaImage::from_fn(64, 48, |x, y| {
            let a = if x < 16 && y < 16 { 0 } else { 255 };
            Rgba([(x * 4) as u8, (y * 5) as u8, 128, a])
        });
        let p = dir.join("sample.png");
        img.save(&p).unwrap();
        p
    }

    fn run(registry: &Registry, input: &Path, op: &OperationDescriptor, options: OptionValues) -> Result<PathBuf> {
        let ext = op.output.unwrap().extension();
        let target = output::target_path(input, ext, op.output_suffix, &OutputPolicy::NextToSource);
        let out = output::reserve(target)?;
        let inputs = [input.to_path_buf()];
        let ctx = JobContext {
            operation_id: &op.id,
            inputs: &inputs,
            output: out.clone(),
            options: &options,
            progress: &|_f: f32| {},
            cancel: &CancelToken::new(),
            modules: &|_: &str| None,
        };
        registry.run(&ctx).map(|_| out)
    }

    fn registry() -> Registry {
        let mut r = Registry::new();
        r.register(ImageConverter);
        r
    }

    #[test]
    fn png_converts_to_every_writable_format() {
        let dir = temp_dir("all");
        let src = sample_png(&dir);
        let r = registry();
        for op in r.operations_for(Format::Png).iter().filter(|o| o.category == OperationCategory::Convert) {
            let out = run(&r, &src, op, OptionValues::new()).unwrap_or_else(|e| panic!("{}: {e}", op.id));
            let written = std::fs::read(&out).unwrap();
            assert!(written.len() > 16, "{} produced an empty file", op.id);
            let expected = op.output.unwrap();
            assert_eq!(format::detect_bytes(&written).or(format::detect_extension(&out)), Some(expected), "{}", op.id);
            // Everything except AVIF (no decoder yet) and ICO (resized) must round-trip with its size.
            if !matches!(expected, Format::Avif | Format::Ico) {
                let back = image::open(&out).unwrap_or_else(|e| panic!("{}: {e}", op.id));
                assert_eq!((back.width(), back.height()), (64, 48), "{}", op.id);
            }
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn jpeg_flattens_transparency_on_white() {
        let dir = temp_dir("flatten");
        let src = sample_png(&dir);
        let r = registry();
        let op = r.describe(Format::Png, "image.convert.jpg").unwrap();
        let out = run(&r, &src, &op, OptionValues::new()).unwrap();
        let px = image::open(out).unwrap().to_rgb8().get_pixel(2, 2).0;
        assert!(px.iter().all(|&c| c > 240), "transparent area should be white, got {px:?}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn resize_keeps_ratio_and_suffixes_name() {
        let dir = temp_dir("resize");
        let src = sample_png(&dir);
        let r = registry();
        let op = r.describe(Format::Png, "image.resize").unwrap();
        let mut o = OptionValues::new();
        o.insert("width".into(), 32.into());
        let out = run(&r, &src, &op, o).unwrap();
        assert_eq!(out.file_name().unwrap(), "sample-resized.png");
        assert_eq!(image::image_dimensions(&out).unwrap(), (32, 24));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn png_compressions_produce_valid_files() {
        let dir = temp_dir("compress");
        let src = sample_png(&dir);
        let r = registry();
        for id in ["image.compress.lossless", "image.compress.smaller"] {
            let op = r.describe(Format::Png, id).unwrap();
            let out = run(&r, &src, &op, OptionValues::new()).unwrap();
            assert_eq!(image::image_dimensions(&out).unwrap(), (64, 48), "{id}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn svg_renders_at_requested_width() {
        let dir = temp_dir("svg");
        let src = dir.join("logo.svg");
        std::fs::write(&src, r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><rect width="100" height="50" fill="#e33"/></svg>"##).unwrap();
        let r = registry();
        let op = r.describe(Format::Svg, "image.convert.png").unwrap();
        let mut o = OptionValues::new();
        o.insert("width".into(), 400.into());
        let out = run(&r, &src, &op, o).unwrap();
        assert_eq!(image::image_dimensions(&out).unwrap(), (400, 200));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
