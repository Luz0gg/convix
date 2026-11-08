//! PDF tools — all built in, nothing to download:
//! merge, split, extract pages, compress (lopdf, pure Rust), images → PDF,
//! and PDF pages → images through the PDF engine that ships with Windows.
//!
//! Operation ids (stable): `pdf.toImages.png`, `pdf.toImages.jpg`, `pdf.compress`,
//! `pdf.split`, `pdf.extract`, `pdf.merge`, `pdf.fromImages`.

mod edit;
mod from_images;
mod pages;
#[cfg(windows)]
mod render;
mod shrink;

use convix_core::{
    Converter, Error, Format, JobContext, MediaKind, OperationCategory, OperationDescriptor,
    OptionSpec, Result, operation::Choice, operation::OptionValuesExt, output,
};
use from_images::PageSize;
use std::path::{Path, PathBuf};

pub struct PdfConverter;

fn op(id: &str, category: OperationCategory, output: Format, label_key: &'static str) -> OperationDescriptor {
    OperationDescriptor {
        id: id.to_owned(),
        converter: "pdf",
        category,
        output: Some(output),
        output_suffix: None,
        label_key,
        multi_input: false,
        requires: &[],
        options: Vec::new(),
    }
}

fn pages_option() -> OptionSpec {
    OptionSpec::Text { id: "pages", label_key: "opt.pages", placeholder_key: "opt.pages.all" }
}

impl Converter for PdfConverter {
    fn id(&self) -> &'static str {
        "pdf"
    }

    fn operations_for(&self, input: Format) -> Vec<OperationDescriptor> {
        if input == Format::Pdf {
            let mut ops = Vec::new();
            if cfg!(windows) {
                let dpi = OptionSpec::Select {
                    id: "dpi",
                    label_key: "opt.resolution",
                    choices: vec![
                        Choice::key("72", "opt.dpi.screen"),
                        Choice::key("150", "opt.dpi.normal"),
                        Choice::key("300", "opt.dpi.print"),
                    ],
                    default: "150",
                };
                for (id, to) in [("pdf.toImages.png", Format::Png), ("pdf.toImages.jpg", Format::Jpeg)] {
                    ops.push(OperationDescriptor {
                        output_suffix: Some("-p"),
                        options: vec![dpi.clone(), pages_option()],
                        ..op(id, OperationCategory::Convert, to, "op.pdfToImages")
                    });
                }
            }
            ops.push(OperationDescriptor {
                output_suffix: Some("-min"),
                options: vec![OptionSpec::Select {
                    id: "level",
                    label_key: "opt.compression",
                    choices: vec![
                        Choice::key("lossless", "opt.pdf.lossless"),
                        Choice::key("balanced", "opt.pdf.balanced"),
                        Choice::key("strong", "opt.pdf.strong"),
                    ],
                    default: "balanced",
                }],
                ..op("pdf.compress", OperationCategory::Compress, Format::Pdf, "op.compressPdf")
            });
            ops.push(OperationDescriptor {
                output_suffix: Some("-p"),
                options: vec![pages_option()],
                ..op("pdf.split", OperationCategory::Extract, Format::Pdf, "op.splitPdf")
            });
            ops.push(OperationDescriptor {
                output_suffix: Some("-extrait"),
                options: vec![OptionSpec::Text { id: "pages", label_key: "opt.pages", placeholder_key: "opt.pages.example" }],
                ..op("pdf.extract", OperationCategory::Extract, Format::Pdf, "op.extractPages")
            });
            ops.push(OperationDescriptor {
                output_suffix: Some("-rot"),
                options: vec![
                    OptionSpec::Select {
                        id: "angle",
                        label_key: "opt.angle",
                        choices: vec![
                            Choice::key("90", "opt.angle.right"),
                            Choice::key("270", "opt.angle.left"),
                            Choice::key("180", "opt.angle.half"),
                        ],
                        default: "90",
                    },
                    pages_option(),
                ],
                ..op("pdf.rotate", OperationCategory::Transform, Format::Pdf, "op.rotatePdf")
            });
            ops.push(OperationDescriptor {
                output_suffix: Some("-modif"),
                options: vec![OptionSpec::Text { id: "pages", label_key: "opt.pagesToDelete", placeholder_key: "opt.pages.example" }],
                ..op("pdf.deletePages", OperationCategory::Transform, Format::Pdf, "op.deletePages")
            });
            ops.push(OperationDescriptor {
                multi_input: true,
                output_suffix: Some("-fusion"),
                ..op("pdf.merge", OperationCategory::Combine, Format::Pdf, "op.mergePdf")
            });
            return ops;
        }
        if input.kind() == MediaKind::Image && convix_image::can_read(input) {
            return vec![OperationDescriptor {
                multi_input: true,
                options: vec![
                    OptionSpec::Select {
                        id: "pageSize",
                        label_key: "opt.pageSize",
                        choices: vec![
                            Choice::literal("a4", "A4"),
                            Choice::literal("letter", "Letter (US)"),
                            Choice::key("image", "opt.pageSize.image"),
                        ],
                        default: "a4",
                    },
                    OptionSpec::Toggle { id: "margin", label_key: "opt.margin", default: true },
                ],
                ..op("pdf.fromImages", OperationCategory::Combine, Format::Pdf, "op.imagesToPdf")
            }];
        }
        Vec::new()
    }

    fn run(&self, ctx: &JobContext) -> Result<Vec<PathBuf>> {
        let first = ctx.inputs.first().ok_or_else(|| Error::Failed("aucun fichier".into()))?;
        match ctx.operation_id {
            "pdf.merge" => {
                let docs = ctx.inputs.iter().map(|p| edit::load(p)).collect::<Result<Vec<_>>>()?;
                ctx.cancel.check()?;
                let mut merged = edit::merge(docs)?;
                write_pdf(&ctx.output, &mut merged)
            }
            "pdf.fromImages" => {
                let margin = if ctx.options.bool_or("margin", true) { from_images::CM } else { 0.0 };
                let size = match ctx.options.str_or("pageSize", "a4") {
                    "image" => PageSize::Image,
                    "letter" => PageSize::Paper { width: from_images::LETTER.0, height: from_images::LETTER.1, margin },
                    _ => PageSize::Paper { width: from_images::A4.0, height: from_images::A4.1, margin },
                };
                let mut doc = from_images::build(ctx.inputs, size, ctx.cancel)?;
                write_pdf(&ctx.output, &mut doc)
            }
            "pdf.compress" => compress(first, ctx),
            "pdf.rotate" => {
                let mut doc = edit::load(first)?;
                edit::prepare_for_subsets(&mut doc); // inherited /Rotate copied into each page
                let count = doc.get_pages().len() as u32;
                let wanted = pages::parse(ctx.options.str_or("pages", ""), count)?;
                let angle = match ctx.options.str_or("angle", "90") {
                    "180" => 180,
                    "270" => 270,
                    _ => 90,
                };
                edit::rotate(&mut doc, &wanted, angle);
                edit::tidy(&mut doc);
                write_pdf(&ctx.output, &mut doc)
            }
            "pdf.deletePages" => {
                let mut doc = edit::load(first)?;
                edit::prepare_for_subsets(&mut doc);
                let count = doc.get_pages().len() as u32;
                let spec = ctx.options.str_or("pages", "");
                if spec.trim().is_empty() {
                    return Err(Error::InvalidOption { name: "pages".into(), reason: "indique les pages à supprimer (ex. 2, 5-7)".into() });
                }
                let remove = pages::parse(spec, count)?;
                let keep: Vec<u32> = (1..=count).filter(|p| !remove.contains(p)).collect();
                if keep.is_empty() {
                    return Err(Error::InvalidOption { name: "pages".into(), reason: "impossible de supprimer toutes les pages".into() });
                }
                let mut part = edit::subset(&doc, &keep);
                write_pdf(&ctx.output, &mut part)
            }
            "pdf.split" | "pdf.extract" => {
                let mut doc = edit::load(first)?;
                edit::prepare_for_subsets(&mut doc);
                let count = doc.get_pages().len() as u32;
                let spec = ctx.options.str_or("pages", "");
                if ctx.operation_id == "pdf.extract" && spec.trim().is_empty() {
                    return Err(Error::InvalidOption { name: "pages".into(), reason: "indique les pages à garder (ex. 1-3, 5)".into() });
                }
                let wanted = pages::parse(spec, count)?;
                if ctx.operation_id == "pdf.extract" {
                    let mut part = edit::subset(&doc, &wanted);
                    return write_pdf(&ctx.output, &mut part);
                }
                let mut outputs = Vec::new();
                for (i, &page) in wanted.iter().enumerate() {
                    ctx.cancel.check()?;
                    let mut part = edit::subset(&doc, &[page]);
                    let target = output::reserve(numbered(&ctx.output, page, count))?;
                    write_pdf(&target, &mut part).inspect_err(|_| output::discard(&target))?;
                    outputs.push(target);
                    ctx.progress.progress((i + 1) as f32 / wanted.len() as f32);
                }
                output::discard(&ctx.output); // the reserved base name was only a pattern
                Ok(outputs)
            }
            #[cfg(windows)]
            id if id.starts_with("pdf.toImages.") => to_images(first, ctx),
            other => Err(Error::UnknownOperation(other.to_owned())),
        }
    }
}

fn write_pdf(target: &Path, doc: &mut lopdf::Document) -> Result<Vec<PathBuf>> {
    output::write_atomic(target, &edit::to_bytes(doc)?)?;
    Ok(vec![target.to_path_buf()])
}

/// `doc-p.png` + page 7 of 120 → `doc-p007.png` (zero-padded so files sort in page order).
/// The base was reserved and may be `doc-p (1).png`: the ` (1)` is dropped, the page files
/// get their own de-duplication (`doc-p007 (1).png`).
fn numbered(base: &Path, page: u32, count: u32) -> PathBuf {
    let width = count.to_string().len();
    let stem = base.file_stem().unwrap_or_default().to_string_lossy();
    let stem = strip_copy_number(&stem);
    let name = match base.extension() {
        Some(ext) => format!("{stem}{page:0width$}.{}", ext.to_string_lossy()),
        None => format!("{stem}{page:0width$}"),
    };
    base.with_file_name(name)
}

/// `"doc-p (3)"` → `"doc-p"`; anything else unchanged.
fn strip_copy_number(stem: &str) -> &str {
    if let Some(open) = stem.strip_suffix(')').and_then(|s| s.rfind(" (")) {
        let digits = &stem[open + 2..stem.len() - 1];
        if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
            return &stem[..open];
        }
    }
    stem
}

fn compress(input: &Path, ctx: &JobContext) -> Result<Vec<PathBuf>> {
    let mut doc = edit::load(input)?;
    let settings = match ctx.options.str_or("level", "balanced") {
        "lossless" => None,
        "strong" => Some(shrink::ShrinkSettings { max_side: 1400, quality: 60.0 }),
        _ => Some(shrink::ShrinkSettings { max_side: 2200, quality: 75.0 }),
    };
    if let Some(s) = settings {
        shrink::shrink_images(&mut doc, &s, ctx.cancel)?;
    }
    edit::tidy(&mut doc);
    let bytes = edit::to_bytes(&mut doc)?;
    // Never hand back a bigger file: an already optimised PDF is kept as it is.
    if bytes.len() as u64 >= std::fs::metadata(input)?.len() {
        output::write_atomic(&ctx.output, &std::fs::read(input)?)?;
    } else {
        output::write_atomic(&ctx.output, &bytes)?;
    }
    Ok(vec![ctx.output.clone()])
}

#[cfg(windows)]
fn to_images(input: &Path, ctx: &JobContext) -> Result<Vec<PathBuf>> {
    use convix_image::{EncodeSettings, encode, image};

    let to = if ctx.operation_id.ends_with(".jpg") { Format::Jpeg } else { Format::Png };
    let dpi = ctx.options.str_or("dpi", "150").parse::<u32>().unwrap_or(150).clamp(36, 600);
    let pdf = render::Rendered::open(input)?;
    let count = pdf.page_count()?;
    let wanted = pages::parse(ctx.options.str_or("pages", ""), count)?;

    let mut settings = EncodeSettings::for_format(to);
    settings.png_optimize = Some(1); // pages are big: a quick lossless pass is enough
    let mut outputs = Vec::new();
    for (i, &page) in wanted.iter().enumerate() {
        ctx.cancel.check()?;
        let png = pdf.page_png(page - 1, dpi)?;
        let img = image::load_from_memory_with_format(&png, image::ImageFormat::Png)
            .map_err(|e| Error::Failed(format!("page {page} : {e}")))?;
        let bytes = encode(&img, None, to, &settings)?;
        let target = output::reserve(numbered(&ctx.output, page, count))?;
        output::write_atomic(&target, &bytes).inspect_err(|_| output::discard(&target))?;
        outputs.push(target);
        ctx.progress.progress((i + 1) as f32 / wanted.len() as f32);
    }
    output::discard(&ctx.output);
    Ok(outputs)
}

#[cfg(test)]
mod tests;

/// First page as a small PNG, for the file cards.
#[cfg(windows)]
pub fn thumbnail(path: &Path, max: u32) -> Result<Vec<u8>> {
    use convix_image::image::{self, imageops::FilterType};
    let pdf = render::Rendered::open(path)?;
    let png = pdf.page_png(0, 48)?;
    let img = image::load_from_memory_with_format(&png, image::ImageFormat::Png)
        .map_err(|e| Error::Failed(e.to_string()))?
        .resize(max, max, FilterType::Triangle);
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).map_err(|e| Error::Failed(e.to_string()))?;
    Ok(out.into_inner())
}
