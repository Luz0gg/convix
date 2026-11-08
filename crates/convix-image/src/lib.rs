//! Image conversions, compression and resizing — built in, no download needed.
//!
//! Operation ids (stable, referenced by presets and the Explorer menu):
//! - `image.convert.<ext>`      e.g. `image.convert.png`, `image.convert.jpg`
//! - `image.compress.lossless`  PNG only (oxipng)
//! - `image.compress.smaller`   PNG (palette), JPEG, WebP (lossy re-encode)
//! - `image.resize`

mod anim;
mod decode;
mod encode;
mod transform;
#[cfg(windows)]
mod wic;

use convix_core::{
    Converter, Error, Format, JobContext, OperationCategory, OperationDescriptor, OptionSpec,
    OptionValues, Result, format, operation::Choice, operation::OptionValuesExt, output,
};
use encode::WRITABLE;
use std::path::PathBuf;

// Reused by the PDF tools (images → PDF, PDF pages → images, image recompression).
pub use decode::{Decoded, decode};
pub use encode::{EncodeSettings, encode, is_opaque};
pub use image;

pub struct ImageConverter;

/// Formats we can read in pure Rust.
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
    // HEIC / AVIF go through the Windows codecs (see wic.rs).
    READABLE.contains(&f) || (cfg!(windows) && matches!(f, Format::Heic | Format::Avif))
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

/// Optional "longest side at most N px" on conversions that already have options
/// (so that one-click conversions like → BMP stay one click).
fn max_side() -> OptionSpec {
    OptionSpec::Number { id: "maxSide", label_key: "opt.maxSide", min: Some(16.0), max: Some(65535.0), default: None, unit: Some("px") }
}

fn convert_options(to: Format) -> Vec<OptionSpec> {
    let mut options = convert_options_base(to);
    if !options.is_empty() && to != Format::Ico {
        options.push(max_side());
    }
    options
}

fn convert_options_base(to: Format) -> Vec<OptionSpec> {
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
                if matches!(input, Format::Gif | Format::Webp) && matches!(to, Format::Gif | Format::Webp) {
                    options.insert(0, OptionSpec::Toggle { id: "keepAnimation", label_key: "opt.keepAnimation", default: true });
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

        // Resizing re-encodes to the source format, so it must be writable (not SVG/HEIC).
        if WRITABLE.contains(&input) && input != Format::Ico {
            ops.push(descriptor(
                "image.resize",
                OperationCategory::Transform,
                input,
                "op.resize",
                "-resized",
                vec![
                    OptionSpec::Number { id: "width", label_key: "opt.width", min: Some(1.0), max: Some(65535.0), default: None, unit: Some("px") },
                    OptionSpec::Number { id: "height", label_key: "opt.height", min: Some(1.0), max: Some(65535.0), default: None, unit: Some("px") },
                    OptionSpec::Toggle { id: "keepRatio", label_key: "opt.keepRatio", default: true },
                    OptionSpec::Number { id: "percent", label_key: "opt.percent", min: Some(1.0), max: Some(1000.0), default: None, unit: Some("%") },
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
        } else if let Some(bytes) = self.convert_animation(action, input, in_fmt, opts)? {
            bytes
        } else {
            let svg_w = if in_fmt == Format::Svg { num("width") } else { None };
            let decoded = decode::decode(input, in_fmt, svg_w)?;
            let icc = decoded.icc.as_deref();
            ctx.cancel.check()?;
            ctx.progress.progress(0.4);

            if let Some(ext) = action.strip_prefix("convert.") {
                let to = Format::from_extension(ext).filter(|f| WRITABLE.contains(f)).ok_or_else(unknown)?;
                // "Longest side at most N px": handy to convert a folder of photos for the web.
                let image = match num("maxSide") {
                    Some(max) if decoded.image.width().max(decoded.image.height()) > max => {
                        std::borrow::Cow::Owned(transform::resize(&decoded.image, Some(max), Some(max), true)?)
                    }
                    _ => std::borrow::Cow::Borrowed(&decoded.image),
                };
                encode::encode(&image, icc, to, &settings_from(opts, to))?
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
                let (width, height) = match num("percent") {
                    // A percentage wins over sizes: "50 %" of every photo of a batch.
                    Some(p) => {
                        let scale = |v: u32| ((f64::from(v) * f64::from(p) / 100.0).round() as u32).max(1);
                        (Some(scale(decoded.image.width())), Some(scale(decoded.image.height())))
                    }
                    None => (num("width"), num("height")),
                };
                let resized = transform::resize(&decoded.image, width, height, opts.bool_or("keepRatio", true))?;
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

impl ImageConverter {
    /// GIF / animated WebP / APNG → GIF or animated WebP, keeping every frame.
    /// `None` when the source is still or the target can't animate: use the normal path.
    fn convert_animation(&self, action: &str, input: &std::path::Path, in_fmt: Format, opts: &OptionValues) -> Result<Option<Vec<u8>>> {
        let Some(to) = action.strip_prefix("convert.").and_then(Format::from_extension) else { return Ok(None) };
        if !matches!(to, Format::Gif | Format::Webp) || !opts.bool_or("keepAnimation", true) {
            return Ok(None);
        }
        let Some(frames) = anim::decode_frames(input, in_fmt)? else { return Ok(None) };
        let s = settings_from(opts, to);
        Ok(Some(match to {
            Format::Gif => anim::encode_gif(frames)?,
            _ => anim::encode_webp(&frames, s.quality, s.lossless)?,
        }))
    }
}

/// Small preview (longest side `max` px) as JPEG, or PNG when transparent.
pub fn thumbnail(path: &std::path::Path, max: u32) -> Result<Vec<u8>> {
    let fmt = format::detect(path)?.filter(|f| can_read(*f)).ok_or(Error::UnsupportedFormat)?;
    let decoded = decode::decode(path, fmt, (fmt == Format::Svg).then_some(max * 2))?;
    let thumb = decoded.image.thumbnail(max, max);
    let mut s = EncodeSettings::for_format(Format::Jpeg);
    s.quality = 80.0;
    s.png_optimize = None;
    let to = if is_opaque(&thumb) { Format::Jpeg } else { Format::Png };
    encode::encode(&thumb, None, to, &s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use convix_core::{CancelToken, Registry, output::OutputPolicy};
    use image::{Rgba, RgbaImage};
    use std::path::Path;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("convix-img-{name}-{}", std::process::id()));
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
    fn convert_with_max_side_and_resize_by_percent() {
        let dir = temp_dir("maxside");
        let src = sample_png(&dir); // 64 × 48
        let r = registry();
        let to_jpg = r.describe(Format::Png, "image.convert.jpg").unwrap();
        let mut o = OptionValues::new();
        o.insert("maxSide".into(), 32.into());
        let out = run(&r, &src, &to_jpg, o).unwrap();
        assert_eq!(image::image_dimensions(&out).unwrap(), (32, 24));
        // Already small enough: untouched (never enlarged).
        let mut o = OptionValues::new();
        o.insert("maxSide".into(), 500.into());
        let out = run(&r, &src, &to_jpg, o).unwrap();
        assert_eq!(image::image_dimensions(&out).unwrap(), (64, 48));

        let resize = r.describe(Format::Png, "image.resize").unwrap();
        let mut o = OptionValues::new();
        o.insert("percent".into(), 50.into());
        o.insert("width".into(), 10.into()); // the percentage wins
        let out = run(&r, &src, &resize, o).unwrap();
        assert_eq!(image::image_dimensions(&out).unwrap(), (32, 24));
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

    /// AVIF → PNG through the Windows codec. Skipped when the AV1 extension isn't installed.
    #[cfg(windows)]
    #[test]
    fn avif_reads_through_windows_codecs() {
        let dir = temp_dir("avif");
        let src = sample_png(&dir);
        let r = registry();
        let to_avif = r.describe(Format::Png, "image.convert.avif").unwrap();
        let avif = run(&r, &src, &to_avif, OptionValues::new()).unwrap();
        let back = r.describe(Format::Avif, "image.convert.png").unwrap();
        match run(&r, &avif, &back, OptionValues::new()) {
            Ok(png) => assert_eq!(image::image_dimensions(&png).unwrap(), (64, 48)),
            Err(Error::Failed(msg)) if msg.contains("Microsoft Store") => eprintln!("skipped: {msg}"),
            Err(e) => panic!("{e}"),
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn animated_gif_stays_animated_through_webp_and_back() {
        use image::{AnimationDecoder, Delay, Frame, codecs::gif::GifEncoder};
        let dir = temp_dir("anim");
        let src = dir.join("loop.gif");
        {
            let mut enc = GifEncoder::new(std::fs::File::create(&src).unwrap());
            let frames = (0..3u8).map(|i| Frame::from_parts(RgbaImage::from_pixel(20, 10, Rgba([i * 80, 0, 0, 255])), 0, 0, Delay::from_numer_denom_ms(100, 1)));
            enc.encode_frames(frames).unwrap();
        }
        let r = registry();
        let to_webp = r.describe(Format::Gif, "image.convert.webp").unwrap();
        let webp = run(&r, &src, &to_webp, OptionValues::new()).unwrap();
        let d = image::codecs::webp::WebPDecoder::new(std::io::BufReader::new(std::fs::File::open(&webp).unwrap())).unwrap();
        assert!(d.has_animation(), "WebP lost its animation");

        let to_gif = r.describe(Format::Webp, "image.convert.gif").unwrap();
        let gif = run(&r, &webp, &to_gif, OptionValues::new()).unwrap();
        let frames = image::codecs::gif::GifDecoder::new(std::io::BufReader::new(std::fs::File::open(&gif).unwrap()))
            .unwrap()
            .into_frames()
            .collect_frames()
            .unwrap();
        assert_eq!(frames.len(), 3);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn thumbnail_is_small() {
        let dir = temp_dir("thumb");
        let src = sample_png(&dir);
        let bytes = thumbnail(&src, 32).unwrap();
        let img = image::load_from_memory(&bytes).unwrap();
        assert_eq!((img.width(), img.height()), (32, 24));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn huge_svg_is_refused_not_rendered() {
        let dir = temp_dir("svgbomb");
        let src = dir.join("bomb.svg");
        std::fs::write(&src, r##"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="10000000"><rect width="1" height="10" fill="#e33"/></svg>"##).unwrap();
        let err = thumbnail(&src, 160).unwrap_err();
        assert!(err.to_string().contains("trop grande"), "{err}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn resize_to_absurd_size_is_refused() {
        let img = image::DynamicImage::ImageRgba8(RgbaImage::new(4, 4));
        assert!(transform::resize(&img, Some(60_000), Some(60_000), false).is_err());
        assert!(transform::resize(&img, Some(60_000), Some(60_000), true).is_err());
        assert!(transform::resize(&img, Some(8), None, true).is_ok());
    }

    #[test]
    fn svg_never_loads_network_or_absolute_images() {
        let dir = temp_dir("svghref");
        RgbaImage::from_pixel(2, 2, Rgba([0, 255, 0, 255])).save(dir.join("ok.png")).unwrap();
        let abs = dir.join("ok.png").display().to_string();
        let src = dir.join("links.svg");
        // A UNC path would make Windows contact "evil-host": must be ignored, not followed.
        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="6" height="2"><image x="0" width="2" height="2" xlink:href="ok.png"/><image x="2" width="2" height="2" xlink:href="\\evil-host\share\x.png"/><image x="4" width="2" height="2" xlink:href="{abs}"/></svg>"##
        );
        std::fs::write(&src, svg).unwrap();
        let r = registry();
        let op = r.describe(Format::Svg, "image.convert.png").unwrap();
        let out = run(&r, &src, &op, OptionValues::new()).unwrap();
        let png = image::open(out).unwrap().to_rgba8();
        assert_eq!(png.get_pixel(0, 0).0, [0, 255, 0, 255], "relative image is drawn");
        assert_eq!(png.get_pixel(3, 0).0[3], 0, "network image is skipped");
        assert_eq!(png.get_pixel(5, 0).0[3], 0, "absolute path is skipped");
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
