//! Replay a run and encode it, picture and sound, to a lossless video file.
//!
//! This is the only artifact the project produces for people who will never
//! read a ledger, so it is deliberately not a new source of truth: the frames
//! come from replaying the same committed input log tier 1 and tier 2 both
//! judge, on the same core, from the same boot. Nothing here can change what
//! the run *is* -- it can only fail to show it.
//!
//! Encoding is `ffmpeg`, driven as a subprocess. It is the one tool in this
//! project that is not in the sandbox image (`apt-get install ffmpeg` on the
//! host); a missing binary is reported as such rather than worked around.
//!
//! ## Why two replays
//!
//! ffmpeg wants its audio input openable at start-up, and the video arrives as
//! a 6 GB stream that must not be spooled to disk. So the run is replayed
//! twice: once to write the audio, once to pipe the picture into an ffmpeg
//! that already has the finished audio file. The emulator is deterministic, so
//! the second pass produces the same frames as the first, and a replay is
//! cheap next to the encode.

use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use frlg_emu::{Emu, InputLog};

/// The GBA's frame rate as an exact rational: the ARM7 clock (16777216 Hz)
/// over the 280896 cycles in a frame. 59.7275 Hz, which is the rate the route
/// docs publish their times at (`docs/defeat-brock/route.md`).
///
/// Not a `decompiled/` claim -- it is a property of the hardware, not of the
/// game -- but it agrees with every published number in this repo.
pub const FRAME_RATE_NUM: u32 = 16_777_216;
pub const FRAME_RATE_DEN: u32 = 280_896;

#[derive(Debug, thiserror::Error)]
pub enum VideoError {
    #[error(transparent)]
    Emu(#[from] frlg_emu::EmuError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(
        "no ffmpeg at {0}: install it on the host (`sudo apt-get install ffmpeg`) \
         or point --ffmpeg at one. It is not in the sandbox image, and this is the \
         one step that needs it."
    )]
    NoFfmpeg(String),
    #[error("ffmpeg exited with {status}; its own error output is above")]
    Ffmpeg { status: String },
    #[error(
        "the ledger was built on boot {expected}, but this machine boots {actual} -- \
         the video would not be the run the ledger describes"
    )]
    Boot { expected: String, actual: String },
    #[error("{0}")]
    Message(String),
}

/// Container and codec pair. Both are lossless; they differ in how widely the
/// container is accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// FFV1 + FLAC in Matroska. The preservation-archive pair, and the
    /// obvious first guess -- but measured on this material it is 3-4x
    /// *larger* than the mp4 below (6.6 MB against 2.2 MB for 1020 frames at
    /// 240x160), because FFV1 is intra-only: a GBA screen that does not move
    /// still costs a full frame every frame. Kept as the archival option and
    /// for wherever H.264 is unwelcome.
    Mkv,
    /// Lossless H.264 in RGB (`libx264rgb -qp 0`) + ALAC in MP4. The default.
    /// Bit-exact -- encoded and decoded back to a byte-identical raw stream to
    /// check, as was ALAC at 65536 Hz -- several times smaller than the FFV1,
    /// since inter-frame prediction pays for the long still stretches a menu
    /// or a battle text box leaves on screen. MP4 is also the container every
    /// upload path documents.
    Mp4,
}

impl Format {
    pub fn parse(name: &str) -> Option<Format> {
        match name {
            "mkv" => Some(Format::Mkv),
            "mp4" => Some(Format::Mp4),
            _ => None,
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Format::Mkv => "mkv",
            Format::Mp4 => "mp4",
        }
    }

    /// One line naming what is inside the file, for the description's
    /// provenance table.
    pub fn describe(self) -> &'static str {
        match self {
            Format::Mkv => "Matroska: FFV1 video (lossless) + FLAC audio (lossless)",
            Format::Mp4 => "MP4: H.264 RGB at qp 0 (lossless) + ALAC audio (lossless)",
        }
    }

    fn codec_args(self) -> Vec<&'static str> {
        match self {
            // level 3 is the frozen FFV1 version; slicecrc puts a checksum on
            // every slice, so a corrupted file reports itself instead of
            // decoding to something plausible. -g 1 keeps every frame
            // intra-coded, which costs little here and makes the file seekable.
            Format::Mkv => vec![
                "-c:v",
                "ffv1",
                "-level",
                "3",
                "-coder",
                "1",
                "-context",
                "1",
                "-g",
                "1",
                "-slices",
                "4",
                "-slicecrc",
                "1",
                "-c:a",
                "flac",
                "-compression_level",
                "12",
            ],
            // libx264rgb, not libx264: the GBA frame is RGB, and going through
            // YUV 4:2:0 -- what plain libx264 defaults to -- is exactly the
            // lossy step this command exists to avoid.
            Format::Mp4 => vec![
                "-c:v",
                "libx264rgb",
                "-qp",
                "0",
                "-preset",
                "veryslow",
                "-c:a",
                "alac",
                "-movflags",
                "+faststart",
            ],
        }
    }
}

pub struct Options {
    pub format: Format,
    /// Integer nearest-neighbour upscale. 1 is the raw 240x160 frame; the
    /// default 4 is still bit-for-bit reversible (every output pixel is a copy
    /// of exactly one input pixel) but survives a video site's own re-encode,
    /// which treats a 240-line source as something to spend no bitrate on.
    pub scale: u32,
    /// Frames of nothing held, appended after the log, so the last thing that
    /// happens is not the last thing on screen. Not part of the run: the frame
    /// count in the title is the log's.
    pub tail_frames: usize,
    /// Truncate the run to this many frames. For checking the pipeline without
    /// waiting for a full encode; the result is not a publishable video.
    pub preview_frames: Option<usize>,
    pub ffmpeg: PathBuf,
    /// The boot the ledger recorded. A video made under a different boot would
    /// be a different run.
    pub expect_boot: String,
    /// Written into the container's metadata.
    pub title: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            format: Format::Mp4,
            scale: 4,
            tail_frames: 120,
            preview_frames: None,
            ffmpeg: PathBuf::from("ffmpeg"),
            expect_boot: String::new(),
            title: None,
        }
    }
}

/// What the encode actually did, for the description's provenance table.
pub struct Report {
    pub path: PathBuf,
    pub bytes: u64,
    /// Video frames written, including `tail_frames`.
    pub frames: usize,
    pub width: u32,
    pub height: u32,
    pub audio_rate: u32,
    pub audio_frames: usize,
    /// Every point the core's own sample rate moved, as (frame, new rate).
    /// The GBA derives it from SOUNDBIAS, which the game may rewrite, so this
    /// is observed rather than assumed.
    pub rate_changes: Vec<(usize, u32)>,
    pub ffmpeg_version: String,
    pub format: Format,
}

impl Report {
    pub fn seconds(&self) -> f64 {
        self.frames as f64 * FRAME_RATE_DEN as f64 / FRAME_RATE_NUM as f64
    }
}

/// Replays `log` and writes one video file. Returns what it wrote.
pub fn encode(
    rom: &Path,
    log: &InputLog,
    out: &Path,
    opts: &Options,
    mut progress: impl FnMut(&str),
) -> Result<Report, VideoError> {
    if opts.scale == 0 {
        return Err(VideoError::Message("--scale must be at least 1".into()));
    }
    let ffmpeg_version = ffmpeg_version(&opts.ffmpeg)?;

    let mut keys: Vec<u16> = log.frames.clone();
    if let Some(limit) = opts.preview_frames {
        keys.truncate(limit);
    }
    keys.extend(std::iter::repeat_n(0u16, opts.tail_frames));

    if let Some(dir) = out.parent() {
        fs::create_dir_all(dir)?;
    }
    let audio_path = out.with_extension("pcm.tmp");
    let audio = capture_audio(rom, &keys, &audio_path, opts, &mut progress)?;

    let result = encode_video(
        rom,
        &keys,
        out,
        &audio,
        opts,
        &ffmpeg_version,
        &mut progress,
    );
    // The PCM is scratch, however the encode went.
    let _ = fs::remove_file(&audio_path);
    result
}

struct Audio {
    path: PathBuf,
    rate: u32,
    channels: u32,
    frames: usize,
    changes: Vec<(usize, u32)>,
}

/// Replay once for sound.
///
/// The core hands out sound through a 0x4000-frame ring buffer that drops what
/// does not fit, so every video frame's worth has to be taken as it is made.
/// The rate it is made at is not fixed: the GBA derives it from SOUNDBIAS's
/// resolution field, and FireRed/LeafGreen do rewrite it during the boot. So
/// the samples are collected at whatever rate they arrive at, and laid onto a
/// single output rate afterwards, against the *video's* clock -- which is what
/// keeps sound and picture together no matter how often the rate moves.
fn capture_audio(
    rom: &Path,
    keys: &[u16],
    path: &Path,
    opts: &Options,
    progress: &mut impl FnMut(&str),
) -> Result<Audio, VideoError> {
    let mut emu = boot(rom, opts)?;
    let channels = emu.audio_channels();
    if channels == 0 {
        return Err(VideoError::Message(
            "the core reports no audio channels; nothing to record".into(),
        ));
    }

    let mut samples: Vec<i16> = Vec::new();
    let mut per_frame: Vec<usize> = Vec::with_capacity(keys.len());
    let mut changes: Vec<(usize, u32)> = Vec::new();
    let mut rate = emu.audio_sample_rate();
    changes.push((0, rate));

    for (index, &frame_keys) in keys.iter().enumerate() {
        emu.step(frame_keys);
        per_frame.push(emu.drain_audio(&mut samples));
        let now = emu.audio_sample_rate();
        if now != rate {
            rate = now;
            changes.push((index, rate));
        }
        if index % 5000 == 4999 {
            progress(&format!("  sound: {} / {} frames", index + 1, keys.len()));
        }
    }

    let out_rate = changes.iter().map(|&(_, r)| r).max().unwrap_or(rate);
    if out_rate == 0 {
        return Err(VideoError::Message(
            "the core reports a sample rate of 0; nothing to record".into(),
        ));
    }

    let written = write_pcm(path, &samples, &per_frame, channels as usize, out_rate)?;
    progress(&format!(
        "  sound: {written} sample frames at {out_rate} Hz, {} rate change(s)",
        changes.len() - 1
    ));
    Ok(Audio {
        path: path.to_path_buf(),
        rate: out_rate,
        channels,
        frames: written,
        changes,
    })
}

/// Lay per-frame chunks of sound, each at whatever rate the core was running
/// at, onto one output rate as raw interleaved s16le.
///
/// The output length of video frame `i` is fixed by the video clock -- the
/// samples that fall inside frames 0..=i at `rate` -- so drift cannot
/// accumulate: sound and picture are back in step at every single frame,
/// whatever the core did in between. Within a frame the mapping is
/// nearest-neighbour, which for the common case (the core's rate already *is*
/// the output rate) copies the chunk unchanged.
fn write_pcm(
    path: &Path,
    samples: &[i16],
    per_frame: &[usize],
    channels: usize,
    rate: u32,
) -> Result<usize, VideoError> {
    let mut out = BufWriter::with_capacity(1 << 20, fs::File::create(path)?);
    let mut src = 0usize;
    let mut emitted = 0usize;

    for (index, &count) in per_frame.iter().enumerate() {
        // Round(  (index+1) * rate * DEN / NUM  ), in integers.
        let numerator = (index as u128 + 1) * rate as u128 * FRAME_RATE_DEN as u128 * 2
            + FRAME_RATE_NUM as u128;
        let cumulative = (numerator / (FRAME_RATE_NUM as u128 * 2)) as usize;
        let want = cumulative.saturating_sub(emitted);

        for step in 0..want {
            if count == 0 {
                // The core produced nothing for this frame. Silence is the
                // honest filler; it has not been observed to happen.
                for _ in 0..channels {
                    out.write_all(&0i16.to_le_bytes())?;
                }
                continue;
            }
            let pick = step * count / want;
            let base = (src + pick) * channels;
            for channel in 0..channels {
                out.write_all(&samples[base + channel].to_le_bytes())?;
            }
        }
        src += count;
        emitted += want;
    }
    out.flush()?;
    Ok(emitted)
}

/// Replay again for picture, straight into ffmpeg's stdin.
fn encode_video(
    rom: &Path,
    keys: &[u16],
    out: &Path,
    audio: &Audio,
    opts: &Options,
    ffmpeg_version: &str,
    progress: &mut impl FnMut(&str),
) -> Result<Report, VideoError> {
    let mut emu = boot(rom, opts)?;
    let width = emu.width();
    let height = emu.height();

    let mut filters = Vec::new();
    if opts.scale > 1 {
        // flags=neighbor: every output pixel is a copy of one input pixel, so
        // the upscale stays reversible. Any interpolating scaler would invent
        // colours the console never drew.
        filters.push(format!(
            "scale=iw*{s}:ih*{s}:flags=neighbor",
            s = opts.scale
        ));
    }
    // The frame arrives as RGBA with a constant alpha; gbrp is the same three
    // channels planar, which is what both encoders take losslessly.
    filters.push("format=gbrp".to_string());

    let mut command = Command::new(&opts.ffmpeg);
    command
        .args(["-hide_banner", "-loglevel", "error", "-stats", "-y"])
        .args(["-f", "rawvideo", "-pix_fmt", "rgba"])
        .args(["-video_size", &format!("{width}x{height}")])
        .args(["-framerate", &format!("{FRAME_RATE_NUM}/{FRAME_RATE_DEN}")])
        .args(["-i", "pipe:0"])
        .args(["-f", "s16le"])
        .args(["-ar", &audio.rate.to_string()])
        .args(["-ac", &audio.channels.to_string()])
        .arg("-i")
        .arg(&audio.path)
        .args(["-map", "0:v:0", "-map", "1:a:0"])
        .args(["-vf", &filters.join(",")])
        .args(opts.format.codec_args());
    if let Some(title) = &opts.title {
        command.args(["-metadata", &format!("title={title}")]);
    }
    command
        .arg(out)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());

    let mut child = command.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            VideoError::NoFfmpeg(opts.ffmpeg.display().to_string())
        } else {
            VideoError::Io(e)
        }
    })?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| VideoError::Message("ffmpeg gave us no stdin".into()))?;

    let mut broken = false;
    for (index, &frame_keys) in keys.iter().enumerate() {
        emu.step(frame_keys);
        if stdin.write_all(&emu.screen_rgba()).is_err() {
            // ffmpeg died; its own message is already on stderr and its exit
            // status below is the error worth reporting.
            broken = true;
            break;
        }
        if index % 5000 == 4999 {
            progress(&format!("  picture: {} / {} frames", index + 1, keys.len()));
        }
    }
    drop(stdin);

    let status = child.wait()?;
    if !status.success() || broken {
        return Err(VideoError::Ffmpeg {
            status: status.to_string(),
        });
    }

    Ok(Report {
        path: out.to_path_buf(),
        bytes: fs::metadata(out)?.len(),
        frames: keys.len(),
        width: width * opts.scale,
        height: height * opts.scale,
        audio_rate: audio.rate,
        audio_frames: audio.frames,
        rate_changes: audio.changes.clone(),
        ffmpeg_version: ffmpeg_version.to_string(),
        format: opts.format,
    })
}

/// A core booted exactly the way the ledger says the run was built.
fn boot(rom: &Path, opts: &Options) -> Result<Emu, VideoError> {
    let mut emu = Emu::new(rom)?;
    let boot = frlg_emu::boot_with_default_bios(&mut emu)?;
    if !opts.expect_boot.is_empty() && boot != opts.expect_boot {
        return Err(VideoError::Boot {
            expected: opts.expect_boot.clone(),
            actual: boot,
        });
    }
    emu.reset();
    emu.clear_audio();
    Ok(emu)
}

/// The first line of `ffmpeg -version`, both as a check that it runs and as
/// something to record in the description.
fn ffmpeg_version(ffmpeg: &Path) -> Result<String, VideoError> {
    let output = Command::new(ffmpeg)
        .arg("-version")
        .output()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => VideoError::NoFfmpeg(ffmpeg.display().to_string()),
            _ => VideoError::Io(e),
        })?;
    if !output.status.success() {
        return Err(VideoError::Message(format!(
            "{} -version exited with {}",
            ffmpeg.display(),
            output.status
        )));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .lines()
        .next()
        .unwrap_or("ffmpeg")
        .split(" Copyright")
        .next()
        .unwrap_or("ffmpeg")
        .trim()
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole point of laying sound on the video's clock: the totals match
    /// the video's own duration, so nothing drifts.
    #[test]
    fn pcm_length_follows_the_video_clock() {
        let dir = std::env::temp_dir().join("frlg-video-test-clock");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.pcm");

        let frames = 600;
        let rate = 65536;
        // 1097 sample frames per video frame is roughly what 65536 Hz gives.
        let per_frame = vec![1097usize; frames];
        let samples = vec![7i16; per_frame.iter().sum::<usize>() * 2];

        let written = write_pcm(&path, &samples, &per_frame, 2, rate).unwrap();
        let expected = (frames as f64 * rate as f64 * FRAME_RATE_DEN as f64 / FRAME_RATE_NUM as f64)
            .round() as usize;
        assert!(
            written.abs_diff(expected) <= 1,
            "{written} sample frames for {frames} video frames, expected ~{expected}"
        );
        assert_eq!(fs::metadata(&path).unwrap().len() as usize, written * 2 * 2);
        fs::remove_dir_all(&dir).ok();
    }

    /// A rate that changes mid-run must not shift the sound against the
    /// picture: the second half is resampled up, not played early.
    #[test]
    fn a_rate_change_does_not_shift_the_sound() {
        let dir = std::env::temp_dir().join("frlg-video-test-rate");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("b.pcm");

        // 100 frames at 32768 Hz, then 100 at 65536 Hz.
        let mut per_frame = vec![549usize; 100];
        per_frame.extend(vec![1097usize; 100]);
        let total: usize = per_frame.iter().sum();
        let samples: Vec<i16> = (0..total * 2).map(|i| (i % 251) as i16).collect();

        let written = write_pcm(&path, &samples, &per_frame, 2, 65536).unwrap();
        let expected =
            (200.0 * 65536.0 * FRAME_RATE_DEN as f64 / FRAME_RATE_NUM as f64).round() as usize;
        assert!(
            written.abs_diff(expected) <= 1,
            "{written} sample frames, expected ~{expected}"
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn formats_round_trip_their_names() {
        assert_eq!(Format::parse("mkv"), Some(Format::Mkv));
        assert_eq!(Format::parse("mp4"), Some(Format::Mp4));
        assert_eq!(Format::parse("webm"), None);
        assert_eq!(Format::Mkv.extension(), "mkv");
    }
}
