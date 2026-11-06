//! Audio conversions via the `ffmpeg` module (downloaded on first use).
//! Also offers "extract audio" on video files — handy for editing.

mod ffmpeg;

use convix_core::{
    Converter, Error, Format, JobContext, MediaKind, OperationCategory, OperationDescriptor,
    OptionSpec, Result, operation::Choice, operation::OptionValuesExt,
};
use ffmpeg::{Depth, Probe};
use std::path::PathBuf;

pub use ffmpeg::probe;

pub struct AudioConverter;

const REQUIRES: &[&str] = &["ffmpeg"];

/// Every format we can write, most useful first.
pub const OUTPUTS: &[Format] = &[
    Format::Mp3,
    Format::Wav,
    Format::Flac,
    Format::M4a,
    Format::Aac,
    Format::Ogg,
    Format::Opus,
    Format::Aiff,
];

/// What "extract audio" offers on a video.
pub const EXTRACT_OUTPUTS: &[Format] = &[Format::Mp3, Format::Wav, Format::Flac, Format::M4a];

fn is_lossless(f: Format) -> bool {
    matches!(f, Format::Wav | Format::Flac | Format::Aiff)
}

fn default_bitrate(to: Format) -> &'static str {
    match to {
        Format::Mp3 => "320",
        Format::Opus => "128",
        _ => "192",
    }
}

fn options_for_output(to: Format) -> Vec<OptionSpec> {
    let mut opts = Vec::new();
    if is_lossless(to) {
        opts.push(OptionSpec::Select {
            id: "bitDepth",
            label_key: "opt.bitDepth",
            choices: vec![
                Choice::key("source", "opt.keepSource"),
                Choice::literal("16", "16 bits"),
                Choice::literal("24", "24 bits"),
            ],
            default: "source",
        });
    } else {
        opts.push(OptionSpec::Select {
            id: "bitrate",
            label_key: "opt.bitrate",
            choices: ["320", "256", "192", "128", "96"]
                .into_iter()
                .zip(["320 kbps", "256 kbps", "192 kbps", "128 kbps", "96 kbps"])
                .map(|(v, l)| Choice::literal(v, l))
                .collect(),
            default: default_bitrate(to),
        });
    }
    // Opus always works at 48 kHz internally: nothing to choose.
    if to != Format::Opus {
        opts.push(OptionSpec::Select {
            id: "sampleRate",
            label_key: "opt.sampleRate",
            choices: vec![
                Choice::key("source", "opt.keepSource"),
                Choice::literal("48000", "48 kHz"),
                Choice::literal("44100", "44,1 kHz"),
            ],
            default: "source",
        });
    }
    opts.push(OptionSpec::Select {
        id: "loudness",
        label_key: "opt.loudness",
        choices: vec![
            Choice::key("off", "opt.loudness.off"),
            Choice::key("-14", "opt.loudness.streaming"),
            Choice::key("-16", "opt.loudness.podcast"),
            Choice::key("-23", "opt.loudness.broadcast"),
        ],
        default: "off",
    });
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
            MediaKind::Video => EXTRACT_OUTPUTS
                .iter()
                .map(|&to| OperationDescriptor {
                    id: format!("audio.extract.{}", to.extension()),
                    converter: self.id(),
                    category: OperationCategory::Extract,
                    output: Some(to),
                    output_suffix: None,
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
        let ffmpeg_exe = ctx.module("ffmpeg")?;
        let input = ctx.inputs.first().ok_or_else(|| Error::Failed("aucun fichier".into()))?;
        let ext = ctx.operation_id.rsplit('.').next().unwrap_or_default();
        let to = Format::from_extension(ext)
            .filter(|f| OUTPUTS.contains(f))
            .ok_or_else(|| Error::UnknownOperation(ctx.operation_id.to_owned()))?;

        let info = ffmpeg::probe(&ffmpeg_exe, input)?;
        let args = encode_args(to, &info, ctx);
        ffmpeg::run(&ffmpeg_exe, input, &ctx.output, &args, info.duration, ctx)?;
        Ok(vec![ctx.output.clone()])
    }
}

/// The ffmpeg arguments placed between the input and the output.
fn encode_args(to: Format, info: &Probe, ctx: &JobContext) -> Vec<String> {
    let o = ctx.options;
    let mut args: Vec<String> = Vec::new();
    let mut push = |a: &[&str]| args.extend(a.iter().map(|s| s.to_string()));

    // First audio track only; drop video/cover art; keep tags (artist, title…).
    push(&["-map", "0:a:0", "-vn", "-sn", "-dn", "-map_metadata", "0"]);

    let loudness = o.str_or("loudness", "off");
    if loudness != "off" {
        push(&["-af", &format!("loudnorm=I={loudness}:TP=-1.5:LRA=11")]);
    }
    match o.str_or("sampleRate", "source") {
        // Opus always encodes at 48 kHz: ffmpeg picks it by itself.
        _ if to == Format::Opus => {}
        "source" if loudness != "off" => {
            // loudnorm works at 192 kHz: bring the result back to the source rate,
            // within what the lossy encoders accept (MP3/AAC/Vorbis: 48 kHz max).
            if let Some(rate) = info.sample_rate {
                let rate = if is_lossless(to) { rate } else { rate.min(48_000) };
                push(&["-ar", &rate.to_string()]);
            }
        }
        "source" => {}
        rate => push(&["-ar", rate]),
    }

    let depth = match o.str_or("bitDepth", "source") {
        "16" => Depth::S16,
        "24" => Depth::S24,
        _ => info.depth_for_lossless(),
    };
    let bitrate = format!("{}k", o.str_or("bitrate", default_bitrate(to)));

    match to {
        Format::Mp3 => push(&["-c:a", "libmp3lame", "-b:a", &bitrate, "-id3v2_version", "3", "-f", "mp3"]),
        Format::M4a => push(&["-c:a", "aac", "-b:a", &bitrate, "-movflags", "+faststart", "-f", "ipod"]),
        Format::Aac => push(&["-c:a", "aac", "-b:a", &bitrate, "-f", "adts"]),
        Format::Ogg => push(&["-c:a", "libvorbis", "-b:a", &bitrate, "-f", "ogg"]),
        Format::Opus => push(&["-c:a", "libopus", "-b:a", &bitrate, "-f", "opus"]),
        Format::Flac => {
            let fmt = if depth == Depth::S16 { "s16" } else { "s32" };
            push(&["-c:a", "flac", "-compression_level", "8", "-sample_fmt", fmt]);
            if depth != Depth::S16 {
                push(&["-bits_per_raw_sample", "24"]);
            }
            push(&["-f", "flac"]);
        }
        Format::Wav => {
            let codec = match depth {
                Depth::S16 => "pcm_s16le",
                Depth::S24 => "pcm_s24le",
                Depth::S32 => "pcm_s32le",
                Depth::F32 => "pcm_f32le",
            };
            // rf64 kicks in automatically past the 4 GB limit of classic WAV.
            push(&["-c:a", codec, "-rf64", "auto", "-f", "wav"]);
        }
        Format::Aiff => {
            let codec = match depth {
                Depth::S16 => "pcm_s16be",
                Depth::S24 => "pcm_s24be",
                Depth::S32 => "pcm_s32be",
                Depth::F32 => "pcm_f32be",
            };
            push(&["-c:a", codec, "-f", "aiff"]);
        }
        _ => unreachable!("filtered by OUTPUTS"),
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use convix_core::{CancelToken, OptionValues};
    use std::path::Path;

    fn ctx_with<'a>(options: &'a OptionValues, cancel: &'a CancelToken, inputs: &'a [PathBuf]) -> JobContext<'a> {
        JobContext {
            operation_id: "audio.convert.mp3",
            inputs,
            output: PathBuf::from("out.mp3"),
            options,
            progress: &|_f: f32| {},
            cancel,
            modules: &|_| None,
        }
    }

    #[test]
    fn audio_and_video_get_operations() {
        let ops = AudioConverter.operations_for(Format::Wav);
        assert!(ops.iter().any(|o| o.id == "audio.convert.mp3"));
        assert!(!ops.iter().any(|o| o.id == "audio.convert.wav"), "no WAV → WAV");
        assert!(ops.iter().all(|o| o.requires == ["ffmpeg"]));
        let ops = AudioConverter.operations_for(Format::Mov);
        assert!(ops.iter().any(|o| o.id == "audio.extract.wav"));
        assert!(AudioConverter.operations_for(Format::Png).is_empty());
    }

    #[test]
    fn missing_ffmpeg_is_reported_as_missing_module() {
        let (options, cancel) = (OptionValues::new(), CancelToken::new());
        let inputs = [PathBuf::from("song.wav")];
        let err = AudioConverter.run(&ctx_with(&options, &cancel, &inputs)).unwrap_err();
        assert!(matches!(err, Error::ModuleMissing(m) if m == "ffmpeg"));
    }

    #[test]
    fn loudness_keeps_source_sample_rate_and_24_bit_survives() {
        let (mut options, cancel) = (OptionValues::new(), CancelToken::new());
        options.insert("loudness".into(), "-14".into());
        let inputs = [PathBuf::from("take.wav")];
        let ctx = ctx_with(&options, &cancel, &inputs);
        let info = ffmpeg::parse_probe(
            "  Duration: 00:01:02.50, start: 0.000000, bitrate: 2304 kb/s\n  Stream #0:0: Audio: pcm_s24le ([1][0][0][0] / 0x0001), 48000 Hz, 2 channels, s32 (24 bit), 2304 kb/s\n",
        );
        let args = encode_args(Format::Wav, &info, &ctx).join(" ");
        assert!(args.contains("loudnorm=I=-14"), "{args}");
        assert!(args.contains("-ar 48000"), "{args}");
        assert!(args.contains("pcm_s24le"), "{args}");
    }

    #[test]
    fn loudness_never_forces_a_rate_lossy_encoders_refuse() {
        let (mut options, cancel) = (OptionValues::new(), CancelToken::new());
        options.insert("loudness".into(), "-16".into());
        let inputs = [PathBuf::from("hires.flac")];
        let ctx = ctx_with(&options, &cancel, &inputs);
        let info = ffmpeg::parse_probe("  Stream #0:0: Audio: flac, 96000 Hz, stereo, s32 (24 bit)\n");
        let mp3 = encode_args(Format::Mp3, &info, &ctx).join(" ");
        assert!(mp3.contains("-ar 48000"), "{mp3}");
        let opus = encode_args(Format::Opus, &info, &ctx).join(" ");
        assert!(!opus.contains("-ar"), "{opus}");
        let flac = encode_args(Format::Flac, &info, &ctx).join(" ");
        assert!(flac.contains("-ar 96000"), "{flac}");
    }

    /// Cancel mid-conversion and 96 kHz + normalization → MP3/Opus, with a real ffmpeg.
    #[test]
    fn cancels_and_handles_hires_for_real_when_ffmpeg_is_available() {
        let Some(exe) = std::env::var_os("CONVIX_FFMPEG").map(PathBuf::from) else {
            eprintln!("skipped: set CONVIX_FFMPEG");
            return;
        };
        let dir = std::env::temp_dir().join(format!("convix-audio-cancel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let locate = |id: &str| (id == "ffmpeg").then(|| exe.clone());

        let hires = dir.join("hires.flac");
        let status = std::process::Command::new(&exe)
            .args(["-y", "-loglevel", "error", "-f", "lavfi", "-i", "sine=frequency=440:duration=600:sample_rate=96000", "-ac", "2", "-c:a", "flac"])
            .arg(&hires)
            .status()
            .unwrap();
        assert!(status.success());
        let inputs = [hires.clone()];

        // 10 minutes of audio normalized to MP3, cancelled after 300 ms.
        let cancel = CancelToken::new();
        let mut options = OptionValues::new();
        options.insert("loudness".into(), "-14".into());
        let out = dir.join("hires.mp3");
        std::fs::write(&out, b"").unwrap(); // the reserved placeholder
        let ctx = JobContext {
            operation_id: "audio.convert.mp3",
            inputs: &inputs,
            output: out.clone(),
            options: &options,
            progress: &|_f: f32| {},
            cancel: &cancel,
            modules: &locate,
        };
        let started = std::time::Instant::now();
        let err = std::thread::scope(|s| {
            s.spawn(|| {
                std::thread::sleep(std::time::Duration::from_millis(300));
                cancel.cancel();
            });
            AudioConverter.run(&ctx).unwrap_err()
        });
        assert!(matches!(err, Error::Cancelled), "{err}");
        assert!(started.elapsed().as_secs() < 5, "cancel took {:?}", started.elapsed());
        let leftovers: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name()).filter(|n| n.to_string_lossy().ends_with(".part")).collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");

        // Short 96 kHz excerpt: normalization to MP3 and Opus must succeed.
        let short = dir.join("short96.flac");
        ffmpeg::generate_test_file(&exe, &short, &["-ar", "96000", "-c:a", "flac"]).unwrap();
        let inputs = [short];
        for (id, ext) in [("audio.convert.mp3", "mp3"), ("audio.convert.opus", "opus")] {
            let out = dir.join(format!("short96.{ext}"));
            let cancel = CancelToken::new();
            let ctx = JobContext {
                operation_id: id,
                inputs: &inputs,
                output: out.clone(),
                options: &options,
                progress: &|_f: f32| {},
                cancel: &cancel,
                modules: &locate,
            };
            AudioConverter.run(&ctx).unwrap_or_else(|e| panic!("{id}: {e}"));
            assert!(std::fs::metadata(&out).unwrap().len() > 1000, "{id}");
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Full conversions when a real ffmpeg is available:
    /// `CONVIX_FFMPEG=…\ffmpeg.exe cargo test -p convix-audio`
    #[test]
    fn converts_for_real_when_ffmpeg_is_available() {
        let Some(exe) = std::env::var_os("CONVIX_FFMPEG").map(PathBuf::from) else {
            eprintln!("skipped: set CONVIX_FFMPEG");
            return;
        };
        let dir = std::env::temp_dir().join(format!("convix-audio-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("tone.wav");
        ffmpeg::generate_test_tone(&exe, &source).expect("test tone");

        let cancel = CancelToken::new();
        let inputs = [source.clone()];
        let locate = |id: &str| (id == "ffmpeg").then(|| exe.clone());
        for &to in OUTPUTS.iter().filter(|&&f| f != Format::Wav) {
            let id = format!("audio.convert.{}", to.extension());
            let out = dir.join(format!("tone.{}", to.extension()));
            let mut options = OptionValues::new();
            options.insert("loudness".into(), "-16".into());
            let last = std::sync::Mutex::new(0f32);
            let progress = |f: f32| *last.lock().unwrap() = f;
            let ctx = JobContext {
                operation_id: &id,
                inputs: &inputs,
                output: out.clone(),
                options: &options,
                progress: &progress,
                cancel: &cancel,
                modules: &locate,
            };
            AudioConverter.run(&ctx).unwrap_or_else(|e| panic!("{id}: {e}"));
            assert!(std::fs::metadata(&out).unwrap().len() > 1000, "{id}");
            let probed = ffmpeg::probe(&exe, &out).unwrap();
            assert!(probed.duration.is_some_and(|d| (d - 3.0).abs() < 0.2), "{id}: {probed:?}");
            assert_eq!(convix_core::format::detect(&out).unwrap(), Some(to), "{id}");
        }
        assert!(!Path::new(&dir.join("tone.mp3.part")).exists());

        let run = |op: &str, input: &Path, out: &Path| {
            let (options, inputs) = (OptionValues::new(), [input.to_path_buf()]);
            let ctx = JobContext {
                operation_id: op,
                inputs: &inputs,
                output: out.to_path_buf(),
                options: &options,
                progress: &|_f: f32| {},
                cancel: &cancel,
                modules: &locate,
            };
            AudioConverter.run(&ctx).unwrap_or_else(|e| panic!("{op}: {e}"));
        };

        // A 24-bit recording stays 24-bit by default ("same as source").
        let take24 = dir.join("take24.wav");
        ffmpeg::generate_test_file(&exe, &take24, &["-c:a", "pcm_s24le"]).unwrap();
        let flac = dir.join("take24.flac");
        run("audio.convert.flac", &take24, &flac);
        assert_eq!(ffmpeg::probe(&exe, &flac).unwrap().depth, Some(Depth::S24));

        // Extract the sound of a video.
        let clip = dir.join("clip.mp4");
        ffmpeg::generate_test_file(&exe, &clip, &["-f", "lavfi", "-i", "testsrc=size=160x90:rate=10:duration=3", "-c:v", "libx264", "-c:a", "aac", "-shortest"]).unwrap();
        let wav = dir.join("clip.wav");
        run("audio.extract.wav", &clip, &wav);
        let probed = ffmpeg::probe(&exe, &wav).unwrap();
        assert_eq!(probed.depth, Some(Depth::S16), "AAC source → 16-bit WAV");
        assert!(probed.duration.is_some_and(|d| (d - 3.0).abs() < 0.2), "{probed:?}");

        // A video without sound gives a clear message, no output.
        let mute = dir.join("mute.mp4");
        ffmpeg::generate_test_file(&exe, &mute, &["-f", "lavfi", "-i", "testsrc=size=160x90:rate=10:duration=1", "-an", "-c:v", "libx264"]).unwrap();
        let err = ffmpeg::probe(&exe, &mute).unwrap_err().to_string();
        assert!(err.contains("piste audio"), "{err}");

        let _ = std::fs::remove_dir_all(dir);
    }
}
