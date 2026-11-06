//! Audio conversions via the `ffmpeg` module (downloaded on first use).
//! Also offers "extract audio" on video files — handy for editing.

use convertio_core::{
    Converter, Error, Format, JobContext, MediaKind, OperationCategory, OperationDescriptor,
    OptionSpec, Result, operation::Choice,
};
use std::path::PathBuf;

pub struct AudioConverter;

const REQUIRES: &[&str] = &["ffmpeg"];

const OUTPUTS: &[Format] = &[
    Format::Mp3,
    Format::Wav,
    Format::Flac,
    Format::Aac,
    Format::M4a,
    Format::Ogg,
    Format::Opus,
];

fn is_lossless(f: Format) -> bool {
    matches!(f, Format::Wav | Format::Flac)
}

fn options_for_output(to: Format) -> Vec<OptionSpec> {
    let mut opts = Vec::new();
    if !is_lossless(to) {
        opts.push(OptionSpec::Select {
            id: "bitrate",
            label_key: "opt.bitrate",
            choices: ["320", "256", "192", "128", "96"]
                .into_iter()
                .zip(["320 kbps", "256 kbps", "192 kbps", "128 kbps", "96 kbps"])
                .map(|(v, l)| Choice::literal(v, l))
                .collect(),
            default: "192",
        });
    }
    opts.push(OptionSpec::Select {
        id: "sampleRate",
        label_key: "opt.sampleRate",
        choices: vec![
            Choice::key("source", "opt.keepSource"),
            Choice::literal("48000", "48 kHz"),
            Choice::literal("44100", "44.1 kHz"),
        ],
        default: "source",
    });
    opts.push(OptionSpec::Toggle { id: "normalize", label_key: "opt.normalize", default: false });
    opts
}

impl Converter for AudioConverter {
    fn id(&self) -> &'static str {
        "audio"
    }

    fn operations_for(&self, input: Format) -> Vec<OperationDescriptor> {
        match input.kind() {
            MediaKind::Audio => OUTPUTS
                .iter()
                .filter(|&&to| to != input)
                .map(|&to| {
                    OperationDescriptor::convert(self.id(), to)
                        .with_options(options_for_output(to))
                        .requiring(REQUIRES)
                })
                .collect(),
            MediaKind::Video => [Format::Mp3, Format::Wav, Format::Flac, Format::M4a]
                .into_iter()
                .map(|to| OperationDescriptor {
                    id: format!("audio.extract.{}", to.extension()),
                    converter: self.id(),
                    category: OperationCategory::Extract,
                    output: Some(to),
                    label_key: "op.extractAudio",
                    multi_input: false,
                    requires: REQUIRES,
                    options: options_for_output(to),
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    fn run(&self, ctx: &JobContext) -> Result<Vec<PathBuf>> {
        let _ffmpeg = ctx.module("ffmpeg")?;
        Err(Error::NotImplemented("audio conversions (Phase 2)"))
    }
}
