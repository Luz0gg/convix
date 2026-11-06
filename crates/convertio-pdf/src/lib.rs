//! PDF tools. Merge/split/compress are pure Rust; rendering pages to images
//! needs the `pdfium` module.

use convertio_core::{
    Converter, Error, Format, JobContext, MediaKind, OperationCategory, OperationDescriptor,
    OptionSpec, Result, operation::Choice,
};
use std::path::PathBuf;

pub struct PdfConverter;

fn op(id: &str, category: OperationCategory, output: Format, label_key: &'static str) -> OperationDescriptor {
    OperationDescriptor {
        id: id.to_owned(),
        converter: "pdf",
        category,
        output: Some(output),
        output_suffix: None, // TODO(Phase 3): "-min" for compress, "-merged" for merge…
        label_key,
        multi_input: false,
        requires: &[],
        options: Vec::new(),
    }
}

impl Converter for PdfConverter {
    fn id(&self) -> &'static str {
        "pdf"
    }

    fn operations_for(&self, input: Format) -> Vec<OperationDescriptor> {
        if input == Format::Pdf {
            let dpi = OptionSpec::Select {
                id: "dpi",
                label_key: "opt.resolution",
                choices: vec![
                    Choice::literal("72", "72 DPI"),
                    Choice::literal("150", "150 DPI"),
                    Choice::literal("300", "300 DPI"),
                ],
                default: "150",
            };
            return vec![
                OperationDescriptor { requires: &["pdfium"], options: vec![dpi.clone()], ..op("pdf.toImages.png", OperationCategory::Convert, Format::Png, "op.pdfToImages") },
                OperationDescriptor { requires: &["pdfium"], options: vec![dpi], ..op("pdf.toImages.jpg", OperationCategory::Convert, Format::Jpeg, "op.pdfToImages") },
                op("pdf.compress", OperationCategory::Compress, Format::Pdf, "op.compressPdf"),
                OperationDescriptor { multi_input: true, ..op("pdf.merge", OperationCategory::Combine, Format::Pdf, "op.mergePdf") },
                op("pdf.split", OperationCategory::Extract, Format::Pdf, "op.splitPdf"),
            ];
        }
        if input.kind() == MediaKind::Image && input != Format::Svg {
            return vec![OperationDescriptor {
                multi_input: true,
                ..op("pdf.fromImages", OperationCategory::Combine, Format::Pdf, "op.imagesToPdf")
            }];
        }
        Vec::new()
    }

    fn run(&self, _ctx: &JobContext) -> Result<Vec<PathBuf>> {
        Err(Error::NotImplemented("PDF tools (Phase 3)"))
    }
}
