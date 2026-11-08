//! Video conversions via the `ffmpeg` module: MP4 (H.264), WebM (VP9), GIF, and
//! "compress the video" (smaller MP4, optionally lower resolution).
//!
//! Operation ids (stable): `video.convert.mp4`, `video.convert.webm`, `video.convert.gif`,
//! `video.compress`.

use crate::ffmpeg::{self, Probe};
use convix_core::{
    Converter, Error, Format, JobContext, MediaKind, OperationCategory, OperationDescriptor,
    OptionSpec, Result, operation::Choice, operation::OptionValuesExt,
};
use std::path::PathBuf;

pub struct VideoConverter;

const REQUIRES: &[&str] = &["ffmpeg"];

fn quality() -> OptionSpec {
    OptionSpec::Select {
        id: "quality",
        label_key: "opt.videoQuality",
        choices: vec![
            Choice::key("high", "opt.videoQuality.high"),
            Choice::key("balanced", "opt.videoQuality.balanced"),
            Choice::key("small", "opt.videoQuality.small"),
        ],
        default: "balanced",
    }
}

fn max_size(default: &'static str) -> OptionSpec {
    OptionSpec::Select {
        id: "maxSize",
        label_key: "opt.maxSize",
        choices: vec![
            Choice::key("source", "opt.keepSource"),
            Choice::literal("2160", "4K"),
            Choice::literal("1080", "1080p"),
            Choice::literal("720", "720p"),
            Choice::literal("480", "480p"),
        ],
        default,
    }
}

fn gif_options() -> Vec<OptionSpec> {
    vec![
        OptionSpec::Select {
            id: "width",
            label_key: "opt.gifWidth",
            choices: ["320", "480", "640", "800"].into_iter().zip(["320 px", "480 px", "640 px", "800 px"]).map(|(v, l)| Choice::literal(v, l)).collect(),
            default: "480",
        },
        OptionSpec::Select {
            id: "fps",
            label_key: "opt.fps",
            choices: ["10", "15", "24"].into_iter().zip(["10 i/s", "15 i/s", "24 i/s"]).map(|(v, l)| Choice::literal(v, l)).collect(),
            default: "15",
        },
    ]
}

impl Converter for VideoConverter {
    fn id(&self) -> &'static str {
        "video"
    }

    fn operations_for(&self, input: Format) -> Vec<OperationDescriptor> {
        if input.kind() != MediaKind::Video {
            return Vec::new();
        }
        let mut ops = Vec::new();
        for to in [Format::Mp4, Format::Webm] {
            if to != input {
                ops.push(OperationDescriptor::convert(self.id(), to).with_options(vec![quality(), max_size("source")]).requiring(REQUIRES));
            }
        }
        ops.push(OperationDescriptor::convert(self.id(), Format::Gif).with_options(gif_options()).requiring(REQUIRES));
        ops.push(OperationDescriptor {
            id: "video.compress".into(),
            converter: "video",
            category: OperationCategory::Compress,
            output: Some(Format::Mp4),
            output_suffix: Some("-min"),
            label_key: "op.compressVideo",
            multi_input: false,
            requires: REQUIRES,
            options: vec![quality(), max_size("1080")],
        });
        ops
    }

    fn run(&self, ctx: &JobContext) -> Result<Vec<PathBuf>> {
        let exe = ctx.module("ffmpeg")?;
        let input = ctx.inputs.first().ok_or_else(|| Error::Failed("aucun fichier".into()))?;
        let info = ffmpeg::probe_media(&exe, input)?;
        if !info.has_video {
            return Err(Error::Failed("ce fichier ne contient pas d'image vidéo".into()));
        }
        let args = match ctx.operation_id {
            "video.convert.mp4" | "video.compress" => mp4_args(&info, ctx),
            "video.convert.webm" => webm_args(&info, ctx),
            "video.convert.gif" => gif_args(ctx),
            other => return Err(Error::UnknownOperation(other.to_owned())),
        };
        ffmpeg::run(&exe, input, &ctx.output, &args, info.duration, ctx)?;
        Ok(vec![ctx.output.clone()])
    }
}

fn strings(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

/// `-vf scale=W:H` so that the **short** side is at most `limit` (portrait videos too),
/// with even sizes (required by H.264 / VP9 in 4:2:0). Nothing when already small enough.
fn scale_filter(info: &Probe, limit: Option<u32>) -> Option<String> {
    let limit = limit?;
    match (info.width, info.height) {
        (Some(w), Some(h)) => {
            let short = w.min(h);
            if short <= limit {
                return None;
            }
            let k = f64::from(limit) / f64::from(short);
            let even = |v: u32| (((f64::from(v) * k) / 2.0).round() as u32 * 2).max(2);
            Some(format!("scale={}:{}:flags=lanczos", even(w), even(h)))
        }
        // Size unknown: limit the height, keep the ratio.
        _ => Some(format!("scale=-2:'min({limit},ih)':flags=lanczos")),
    }
}

fn limit_of(ctx: &JobContext, default: &str) -> Option<u32> {
    ctx.options.str_or("maxSize", default).parse().ok()
}

/// Common to MP4/WebM: first video track, optional first audio track, tags kept,
/// subtitles and data dropped (they rarely survive a container change).
fn mapping(info: &Probe) -> Vec<String> {
    let mut args = strings(&["-map", "0:v:0", "-map_metadata", "0", "-sn", "-dn"]);
    if info.has_audio {
        args.extend(strings(&["-map", "0:a:0"]));
    }
    args
}

fn mp4_args(info: &Probe, ctx: &JobContext) -> Vec<String> {
    let default_limit = if ctx.operation_id == "video.compress" { "1080" } else { "source" };
    // CRF: lower = better quality / bigger file. 23 is x264's own default.
    let crf = match (ctx.options.str_or("quality", "balanced"), ctx.operation_id) {
        ("high", _) => "19",
        ("small", _) => "28",
        (_, "video.compress") => "24",
        _ => "21",
    };
    let mut args = mapping(info);
    if let Some(vf) = scale_filter(info, limit_of(ctx, default_limit)) {
        args.extend(["-vf".into(), vf]);
    }
    args.extend(strings(&["-c:v", "libx264", "-preset", "medium", "-crf", crf, "-pix_fmt", "yuv420p", "-profile:v", "high"]));
    if info.has_audio {
        let bitrate = if ctx.options.str_or("quality", "balanced") == "small" { "128k" } else { "192k" };
        args.extend(strings(&["-c:a", "aac", "-b:a", bitrate]));
    }
    // Plays while downloading / in a browser; "mp4" muxer.
    args.extend(strings(&["-movflags", "+faststart", "-f", "mp4"]));
    args
}

fn webm_args(info: &Probe, ctx: &JobContext) -> Vec<String> {
    let crf = match ctx.options.str_or("quality", "balanced") {
        "high" => "28",
        "small" => "38",
        _ => "33",
    };
    let mut args = mapping(info);
    if let Some(vf) = scale_filter(info, limit_of(ctx, "source")) {
        args.extend(["-vf".into(), vf]);
    }
    // Constant quality VP9 (-b:v 0), multi-threaded, "good" speed/quality trade-off.
    args.extend(strings(&["-c:v", "libvpx-vp9", "-crf", crf, "-b:v", "0", "-deadline", "good", "-cpu-used", "4", "-row-mt", "1", "-pix_fmt", "yuv420p"]));
    if info.has_audio {
        args.extend(strings(&["-c:a", "libopus", "-b:a", "128k"]));
    }
    args.extend(strings(&["-f", "webm"]));
    args
}

/// GIF with its own optimized palette (much better than ffmpeg's default 256 web colors).
fn gif_args(ctx: &JobContext) -> Vec<String> {
    let width: u32 = ctx.options.str_or("width", "480").parse().unwrap_or(480).clamp(16, 1920);
    let fps: u32 = ctx.options.str_or("fps", "15").parse().unwrap_or(15).clamp(1, 50);
    let graph = format!(
        "[0:v:0]fps={fps},scale={width}:-2:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle"
    );
    vec!["-filter_complex".into(), graph, "-an".into(), "-loop".into(), "0".into(), "-f".into(), "gif".into()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use convix_core::{CancelToken, OptionValues};

    fn ctx_with<'a>(op: &'a str, options: &'a OptionValues, cancel: &'a CancelToken, inputs: &'a [PathBuf]) -> JobContext<'a> {
        JobContext {
            operation_id: op,
            inputs,
            output: PathBuf::from("out.mp4"),
            options,
            progress: &|_f: f32| {},
            cancel,
            modules: &|_| None,
        }
    }

    fn probe(w: u32, h: u32) -> Probe {
        Probe { has_video: true, has_audio: true, width: Some(w), height: Some(h), ..Default::default() }
    }

    #[test]
    fn operations_only_for_videos() {
        let ids: Vec<_> = VideoConverter.operations_for(Format::Mov).into_iter().map(|o| o.id).collect();
        assert_eq!(ids, ["video.convert.mp4", "video.convert.webm", "video.convert.gif", "video.compress"]);
        let on_mp4: Vec<_> = VideoConverter.operations_for(Format::Mp4).into_iter().map(|o| o.id).collect();
        assert!(!on_mp4.contains(&"video.convert.mp4".to_string()), "no MP4 → MP4 (that is \"compress\")");
        assert!(VideoConverter.operations_for(Format::Mp3).is_empty());
        assert!(VideoConverter.operations_for(Format::Mov).iter().all(|o| o.requires == ["ffmpeg"]));
    }

    #[test]
    fn short_side_is_limited_and_even() {
        assert_eq!(scale_filter(&probe(3840, 2160), Some(1080)).unwrap(), "scale=1920:1080:flags=lanczos");
        // Portrait phone video: the width (short side) is limited, not the height.
        assert_eq!(scale_filter(&probe(1080, 1920), Some(720)).unwrap(), "scale=720:1280:flags=lanczos");
        assert_eq!(scale_filter(&probe(1279, 721), Some(480)).unwrap(), "scale=852:480:flags=lanczos");
        assert!(scale_filter(&probe(1280, 720), Some(1080)).is_none(), "never upscaled");
        assert!(scale_filter(&probe(1280, 720), None).is_none());
    }

    #[test]
    fn compress_defaults_to_1080p_and_keeps_audio() {
        let (options, cancel, inputs) = (OptionValues::new(), CancelToken::new(), [PathBuf::from("clip.mov")]);
        let args = mp4_args(&probe(3840, 2160), &ctx_with("video.compress", &options, &cancel, &inputs)).join(" ");
        assert!(args.contains("scale=1920:1080"), "{args}");
        assert!(args.contains("-crf 24") && args.contains("-c:a aac"), "{args}");
        let mut silent = probe(640, 360);
        silent.has_audio = false;
        let args = mp4_args(&silent, &ctx_with("video.convert.mp4", &options, &cancel, &inputs)).join(" ");
        assert!(!args.contains("0:a:0") && !args.contains("-c:a"), "{args}");
    }

    /// Real conversions when an ffmpeg is available: `CONVIX_FFMPEG=…\ffmpeg.exe cargo test -p convix-audio`
    #[test]
    fn converts_videos_for_real_when_ffmpeg_is_available() {
        let Some(exe) = std::env::var_os("CONVIX_FFMPEG").map(PathBuf::from) else {
            eprintln!("skipped: set CONVIX_FFMPEG");
            return;
        };
        let dir = std::env::temp_dir().join(format!("convix-video-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let clip = dir.join("clip.mov");
        let status = std::process::Command::new(&exe)
            .args(["-y", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc=size=1280x720:rate=25:duration=2", "-f", "lavfi", "-i", "sine=frequency=440:duration=2", "-c:v", "libx264", "-c:a", "aac", "-shortest"])
            .arg(&clip)
            .status()
            .unwrap();
        assert!(status.success());
        let inputs = [clip];
        let locate = |id: &str| (id == "ffmpeg").then(|| exe.clone());
        let cancel = CancelToken::new();
        for (op, ext, size) in [("video.convert.mp4", "mp4", 720), ("video.compress", "mp4", 480), ("video.convert.webm", "webm", 720), ("video.convert.gif", "gif", 0)] {
            let mut options = OptionValues::new();
            if op == "video.compress" {
                options.insert("maxSize".into(), "480".into());
            }
            let out = dir.join(format!("{}.{ext}", op.replace('.', "-")));
            let ctx = JobContext {
                operation_id: op,
                inputs: &inputs,
                output: out.clone(),
                options: &options,
                progress: &|_f: f32| {},
                cancel: &cancel,
                modules: &locate,
            };
            VideoConverter.run(&ctx).unwrap_or_else(|e| panic!("{op}: {e}"));
            assert!(std::fs::metadata(&out).unwrap().len() > 1000, "{op}");
            let probed = ffmpeg::probe_media(&exe, &out).unwrap();
            assert!(probed.has_video, "{op}");
            if size > 0 {
                assert_eq!(probed.height, Some(size), "{op}: {probed:?}");
                assert!(probed.has_audio, "{op} kept the sound");
            } else {
                assert_eq!(probed.width, Some(480), "{op}: {probed:?}");
            }
        }
        let _ = std::fs::remove_dir_all(dir);
    }
}
