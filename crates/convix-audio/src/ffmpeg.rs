//! Driving `ffmpeg.exe`: probing a file, running a conversion with real progress
//! (`-progress pipe:1`), cancellation (the process is killed) and readable errors.

use convix_core::{Error, JobContext, Result, output};
use std::{
    io::{BufRead, BufReader, Read},
    path::Path,
    process::{Command, Stdio},
    sync::mpsc,
    time::Duration,
};

/// Sample format of the source, to keep lossless outputs as precise as the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Depth {
    S16,
    S24,
    S32,
    F32,
}

#[derive(Debug, Clone, Default)]
pub struct Probe {
    /// Seconds, when the container knows it.
    pub duration: Option<f64>,
    pub has_audio: bool,
    pub codec: String,
    pub sample_rate: Option<u32>,
    /// `None` for lossy sources (decoded as float, but 16 bits is what they really hold).
    pub depth: Option<Depth>,
}

impl Probe {
    /// Bit depth to write for WAV/FLAC/AIFF when the user keeps "same as source".
    pub fn depth_for_lossless(&self) -> Depth {
        self.depth.unwrap_or(Depth::S16)
    }
}

const LOSSY_CODECS: &[&str] = &[
    "mp3", "mp2", "mp1", "aac", "vorbis", "opus", "ac3", "eac3", "dts", "dca", "wmav1", "wmav2", "wmapro",
    "wmavoice", "amr_nb", "amr_wb", "cook", "atrac3", "atrac3p", "ra_144", "ra_288", "qdm2", "nellymoser", "speex",
    "gsm", "gsm_ms",
];

/// Read what `ffmpeg -i <file>` prints about the file (it exits with an error, that is expected).
pub fn parse_probe(stderr: &str) -> Probe {
    let mut probe = Probe::default();
    for line in stderr.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("Duration:") {
            probe.duration = rest.split(',').next().and_then(|t| parse_clock(t.trim()));
        } else if !probe.has_audio && line.starts_with("Stream #") {
            let Some((_, audio)) = line.split_once(": Audio: ") else { continue };
            probe.has_audio = true;
            let fields: Vec<&str> = audio.split(", ").map(str::trim).collect();
            probe.codec = fields.first().and_then(|c| c.split_whitespace().next()).unwrap_or_default().to_owned();
            probe.sample_rate = fields.iter().find_map(|f| f.strip_suffix(" Hz")?.parse().ok());
            let lossy = LOSSY_CODECS.contains(&probe.codec.as_str());
            probe.depth = if lossy {
                None
            } else if audio.contains("(24 bit)") {
                Some(Depth::S24)
            } else {
                fields.iter().find_map(|f| match f.split_whitespace().next()? {
                    "u8" | "u8p" | "s16" | "s16p" => Some(Depth::S16),
                    "s32" | "s32p" | "s64" | "s64p" => Some(Depth::S32),
                    "flt" | "fltp" | "dbl" | "dblp" => Some(Depth::F32),
                    _ => None,
                })
            };
        }
    }
    probe
}

/// `"01:02:03.50"` → 3723.5 seconds.
fn parse_clock(t: &str) -> Option<f64> {
    let mut parts = t.split(':');
    let (h, m, s) = (parts.next()?, parts.next()?, parts.next()?);
    Some(h.parse::<f64>().ok()? * 3600.0 + m.parse::<f64>().ok()? * 60.0 + s.parse::<f64>().ok()?)
}

fn command(exe: &Path) -> Command {
    let mut cmd = Command::new(exe);
    cmd.args(["-hide_banner", "-nostdin"]).stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000; // no black console flashing
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// `file:` keeps ffmpeg from reading odd file names as protocols or options.
fn file_arg(p: &Path) -> String {
    format!("file:{}", p.display())
}

fn spawn_error(e: std::io::Error) -> Error {
    Error::Failed(format!("impossible de lancer FFmpeg ({e}) : réinstallez le module dans les Réglages"))
}

pub fn probe(exe: &Path, input: &Path) -> Result<Probe> {
    let out = command(exe).arg("-i").arg(file_arg(input)).output().map_err(spawn_error)?;
    let probe = parse_probe(&String::from_utf8_lossy(&out.stderr));
    if !probe.has_audio {
        return Err(Error::Failed("ce fichier ne contient pas de piste audio".into()));
    }
    Ok(probe)
}

/// Keep the end of ffmpeg's log: that is where the reason of a failure is.
fn tail_of_log(log: &str) -> String {
    let lines: Vec<&str> = log
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("Stream mapping") && !l.starts_with("Press [q]"))
        .collect();
    lines[lines.len().saturating_sub(2)..].join(" — ")
}

/// Encode into a fresh temporary file next to `output`, then rename it over the reserved name.
///
/// Progress lines are read on their own thread so that cancelling works even when ffmpeg
/// is stuck (slow network share…): the process is killed within ~200 ms.
pub fn run(exe: &Path, input: &Path, output: &Path, args: &[String], duration: Option<f64>, ctx: &JobContext) -> Result<()> {
    let part = output::temp_beside(output)?;

    let spawned = command(exe)
        .args(["-y", "-loglevel", "error", "-progress", "pipe:1", "-nostats"])
        .arg("-i")
        .arg(file_arg(input))
        .args(args)
        .arg(file_arg(&part))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match spawned {
        Ok(c) => c,
        Err(e) => {
            let _ = std::fs::remove_file(&part);
            return Err(spawn_error(e));
        }
    };
    job::attach(&child);

    // Drain stderr on its own thread so a chatty ffmpeg can never block on a full pipe.
    let mut stderr = child.stderr.take().expect("piped");
    let log = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        s
    });
    // Progress: `out_time_us` (or the older, misnamed `out_time_ms`) is in microseconds.
    let (tx, rx) = mpsc::channel::<f64>();
    let stdout = BufReader::new(child.stdout.take().expect("piped"));
    std::thread::spawn(move || {
        for line in stdout.lines() {
            let Ok(line) = line else { break };
            let micros = line.strip_prefix("out_time_us=").or_else(|| line.strip_prefix("out_time_ms="));
            if let Some(us) = micros.and_then(|v| v.parse::<f64>().ok())
                && tx.send(us).is_err() {
                    break;
                }
        }
    });

    let mut cancelled = false;
    let status = loop {
        if ctx.cancel.is_cancelled() {
            cancelled = true;
            let _ = child.kill();
            break child.wait()?;
        }
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(us) => {
                if let Some(total) = duration.filter(|t| *t > 0.0) {
                    ctx.progress.progress((us / 1_000_000.0 / total).clamp(0.0, 1.0) as f32);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            // stdout closed: ffmpeg is finishing.
            Err(mpsc::RecvTimeoutError::Disconnected) => match child.try_wait()? {
                Some(status) => break status,
                None => std::thread::sleep(Duration::from_millis(50)),
            },
        }
    };
    let log = log.join().unwrap_or_default();

    let finish = || -> Result<()> {
        if cancelled || ctx.cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        if !status.success() {
            let reason = tail_of_log(&log);
            return Err(Error::Failed(if reason.is_empty() { "FFmpeg a échoué".into() } else { format!("FFmpeg : {reason}") }));
        }
        output::commit(&part, output)
    };
    let result = finish();
    if result.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    result
}

/// Every ffmpeg we start belongs to one Windows "job" that dies with Convix: if the app is
/// closed or crashes mid-conversion, no ffmpeg.exe keeps running in the background.
#[cfg(windows)]
mod job {
    use std::{os::windows::io::AsRawHandle, process::Child, sync::OnceLock};
    use windows::Win32::{
        Foundation::HANDLE,
        System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation, SetInformationJobObject,
        },
    };

    struct Job(HANDLE);
    // The handle is only used for AssignProcessToJobObject, which is thread-safe.
    unsafe impl Send for Job {}
    unsafe impl Sync for Job {}

    fn job() -> Option<&'static Job> {
        static JOB: OnceLock<Option<Job>> = OnceLock::new();
        JOB.get_or_init(|| unsafe {
            let handle = CreateJobObjectW(None, None).ok()?;
            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
            .ok()?;
            Some(Job(handle)) // never closed: the OS closes it when Convix exits
        })
        .as_ref()
    }

    pub fn attach(child: &Child) {
        if let Some(job) = job() {
            unsafe {
                let _ = AssignProcessToJobObject(job.0, HANDLE(child.as_raw_handle()));
            }
        }
    }
}

#[cfg(not(windows))]
mod job {
    pub fn attach(_: &std::process::Child) {}
}

/// 3 s stereo 440 Hz tone, for tests.
#[cfg(test)]
pub fn generate_test_tone(exe: &Path, dest: &Path) -> std::io::Result<()> {
    let status = command(exe)
        .args(["-y", "-loglevel", "error", "-f", "lavfi", "-i", "sine=frequency=440:duration=3:sample_rate=48000", "-ac", "2"])
        .arg(dest)
        .status()?;
    if status.success() { Ok(()) } else { Err(std::io::Error::other("ffmpeg failed")) }
}

/// 3 s tone plus `extra` arguments (codec, an extra video input…), for tests.
#[cfg(test)]
pub fn generate_test_file(exe: &Path, dest: &Path, extra: &[&str]) -> std::io::Result<()> {
    let status = command(exe)
        .args(["-y", "-loglevel", "error", "-f", "lavfi", "-i", "sine=frequency=440:duration=3:sample_rate=48000"])
        .args(extra)
        .arg(dest)
        .status()?;
    if status.success() { Ok(()) } else { Err(std::io::Error::other(format!("ffmpeg failed for {}", dest.display()))) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_of_a_video_with_aac_audio() {
        let p = parse_probe(
            "Input #0, mov,mp4,m4a,3gp,3g2,mj2, from 'clip.mp4':\n  Duration: 00:03:25.43, start: 0.000000, bitrate: 5000 kb/s\n  Stream #0:0[0x1](und): Video: h264 (High), yuv420p, 1920x1080, 25 fps\n  Stream #0:1[0x2](und): Audio: aac (LC) (mp4a / 0x6134706D), 48000 Hz, stereo, fltp, 317 kb/s (default)\n",
        );
        assert!(p.has_audio);
        assert_eq!(p.codec, "aac");
        assert_eq!(p.sample_rate, Some(48000));
        assert_eq!(p.depth, None);
        assert!((p.duration.unwrap() - 205.43).abs() < 1e-6);
    }

    #[test]
    fn probe_of_lossless_sources() {
        let flac = parse_probe("  Stream #0:0: Audio: flac, 96000 Hz, stereo, s32 (24 bit)\n");
        assert_eq!(flac.depth, Some(Depth::S24));
        let wav16 = parse_probe("  Stream #0:0: Audio: pcm_s16le ([1][0][0][0] / 0x0001), 44100 Hz, 2 channels, s16, 1411 kb/s\n");
        assert_eq!(wav16.depth, Some(Depth::S16));
        let float = parse_probe("  Stream #0:0: Audio: pcm_f32le ([3][0][0][0] / 0x0003), 48000 Hz, stereo, flt, 3072 kb/s\n");
        assert_eq!(float.depth, Some(Depth::F32));
        let silent = parse_probe("  Duration: N/A\n  Stream #0:0: Video: h264\n");
        assert!(!silent.has_audio);
        assert_eq!(silent.duration, None);
    }
}
