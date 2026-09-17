use kukuri_cn_safety_video::VideoExtractConfig;

#[test]
fn samples_are_reproducible_and_cover_the_whole_duration() {
    let config = VideoExtractConfig::default();
    assert_eq!(
        config.sample_times_us(417_000).expect("short"),
        vec![208_500]
    );
    assert_eq!(
        config.sample_times_us(12_000_000).expect("medium"),
        vec![2_000_000, 6_000_000, 10_000_000]
    );
    let expected = vec![
        3_750_000, 11_250_000, 18_750_000, 26_250_000, 33_750_000, 41_250_000, 48_750_000,
        56_250_000,
    ];
    assert_eq!(config.sample_times_us(60_000_000).expect("long"), expected);
    assert_eq!(
        config.sample_times_us(60_000_000).expect("repeat"),
        expected
    );
    assert!(config.sample_times_us(0).is_err());
    assert!(config.sample_times_us(600_000_001).is_err());
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use kukuri_cn_safety::provider::VideoFrameExtractor;

    #[tokio::test]
    async fn bundled_readiness_decodes_both_containers() {
        // Each test owns its scratch root; the default root is left to deployed processes.
        let root = private_tmpfs();
        let extractor = FfmpegVideoExtractor::new(VideoExtractConfig {
            temporary_root: root.path().to_owned(),
            ..Default::default()
        })
        .expect("decoder");
        let frame = extractor
            .readiness_probe()
            .await
            .expect("synthetic MP4/WebM probe");
        assert!(frame.bytes.starts_with(&[0xff, 0xd8]));
        assert!(frame.bytes.len() <= 256 * 1024);
    }
    use kukuri_cn_safety_video::FfmpegVideoExtractor;
    use std::{path::Path, process::Command, sync::Arc, time::Duration};

    fn private_tmpfs() -> tempfile::TempDir {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir_in("/dev/shm").expect("tmpfs");
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700))
            .expect("private scratch");
        root
    }

    fn fixture(root: &Path, format: &str, duration: &str, audio: bool) -> Vec<u8> {
        let path = root.join(format!("fixture.{format}"));
        let mut command = Command::new("/usr/bin/ffmpeg");
        command.args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=160x90:rate=12",
        ]);
        if audio {
            command.args(["-f", "lavfi", "-i", "sine=frequency=440:sample_rate=16000"]);
        }
        command.args([
            "-t",
            duration,
            "-c:v",
            if format == "mp4" {
                "libx264"
            } else {
                "libvpx-vp9"
            },
            "-threads",
            "1",
        ]);
        if format == "mp4" {
            command.args(["-preset", "ultrafast"]);
        } else {
            command.args(["-deadline", "realtime", "-cpu-used", "8"]);
        }
        if audio {
            command.args(["-c:a", if format == "mp4" { "aac" } else { "libopus" }]);
        }
        let output = command
            .arg("-y")
            .arg(&path)
            .output()
            .expect("fixture ffmpeg installed");
        assert!(
            output.status.success(),
            "fixture generation: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::fs::read(path).expect("fixture bytes")
    }
    fn jobs(root: &Path) -> Vec<std::path::PathBuf> {
        std::fs::read_dir(root)
            .expect("scratch root")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".kukuri-video-job-")
            })
            .map(|entry| entry.path())
            .collect()
    }

    #[tokio::test]
    async fn input_decoder_threads_are_bounded() {
        use std::os::unix::fs::PermissionsExt;
        let root = private_tmpfs();
        let programs = tempfile::tempdir().expect("trusted test wrapper directory");
        let wrapper = programs.path().join("ffmpeg-many-threads");
        std::fs::write(
            &wrapper,
            b"#!/bin/sh\nexec /usr/bin/ffmpeg -threads 64 \"$@\" 2>\"$0.stderr\"\n",
        )
        .expect("synthetic decoder thread default");
        std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700))
            .expect("executable wrapper");
        let extractor = FfmpegVideoExtractor::new(VideoExtractConfig {
            ffmpeg: wrapper,
            temporary_root: root.path().to_owned(),
            ..Default::default()
        })
        .expect("bounded extractor");
        for format in ["mp4", "webm"] {
            let bytes = fixture(root.path(), format, "60", false);
            let frames = extractor.extract(&bytes).await.unwrap_or_else(|error| {
                panic!(
                    "64-thread default {format} decode: {error}; synthetic fixture diagnostics: {}",
                    std::fs::read_to_string(programs.path().join("ffmpeg-many-threads.stderr"))
                        .unwrap_or_default()
                )
            });
            assert_eq!(frames.frames.len(), 8);
            assert!(jobs(root.path()).is_empty());
        }
    }

    #[tokio::test]
    async fn mp4_webm_short_audio_and_long_video_extract_without_residue() {
        let root = private_tmpfs();
        let config = VideoExtractConfig {
            temporary_root: root.path().to_owned(),
            ..Default::default()
        };
        let extractor = FfmpegVideoExtractor::new(config).expect("sandboxed extractor");
        for format in ["mp4", "webm"] {
            for (duration, audio, count) in [
                ("0.083333", false, 1),
                ("0.417", false, 1),
                ("12", true, 3),
                ("60", false, 8),
            ] {
                let bytes = fixture(root.path(), format, duration, audio);
                let result = extractor.extract(&bytes).await.unwrap_or_else(|error| {
                    panic!("bounded extraction {format}/{duration}/audio={audio}: {error}")
                });
                assert_eq!(result.frames.len(), count, "{format}/{duration}");
                for frame in result.frames {
                    assert_eq!(frame.content_type, "image/jpeg");
                    assert!(frame.bytes.len() <= 256 * 1024);
                    let decoded = image::load_from_memory(&frame.bytes).expect("real JPEG");
                    assert!(decoded.width() <= 512 && decoded.height() <= 512);
                }
                assert!(jobs(root.path()).is_empty(), "completed job cleanup");
            }
        }
        assert!(extractor.extract(b"broken video").await.is_err());
        assert!(jobs(root.path()).is_empty(), "failed job cleanup");
    }

    #[tokio::test]
    async fn sampled_colors_come_from_beginning_middle_and_end() {
        let root = private_tmpfs();
        let path = root.path().join("colors.mp4");
        let mut command = Command::new("/usr/bin/ffmpeg");
        command.args(["-hide_banner", "-loglevel", "error"]);
        for color in ["red", "green", "blue"] {
            command.args([
                "-f",
                "lavfi",
                "-i",
                &format!("color=c={color}:s=160x90:r=5:d=20"),
            ]);
        }
        let status = command
            .args([
                "-filter_complex",
                "[0:v][1:v][2:v]concat=n=3:v=1:a=0[v]",
                "-map",
                "[v]",
                "-c:v",
                "libx264",
                "-threads",
                "1",
                "-preset",
                "ultrafast",
                "-y",
            ])
            .arg(&path)
            .status()
            .expect("fixture");
        assert!(status.success());
        let extractor = FfmpegVideoExtractor::new(VideoExtractConfig {
            temporary_root: root.path().to_owned(),
            ..Default::default()
        })
        .expect("extractor");
        let result = extractor
            .extract(&std::fs::read(path).expect("bytes"))
            .await
            .expect("extract");
        let dominant: Vec<usize> = result
            .frames
            .iter()
            .map(|frame| {
                let decoded = image::load_from_memory(&frame.bytes)
                    .expect("jpeg")
                    .to_rgb8();
                let p = decoded.get_pixel(10, 10);
                (0..3).max_by_key(|i| p[*i]).expect("RGB")
            })
            .collect();
        assert_eq!(dominant, vec![0, 0, 0, 1, 1, 2, 2, 2]);
    }

    #[tokio::test]
    async fn cancel_reaps_decoder_and_removes_temporary_data() {
        use std::os::unix::fs::PermissionsExt;
        let root = private_tmpfs();
        // Executables cannot live on noexec tmpfs; this contains only a benign test script.
        let executable = tempfile::tempdir().expect("script directory");
        let script = executable.path().join("decoder");
        std::fs::write(
            &script,
            "#!/bin/sh\nprintf '%s' \"$$\" > child.pid\n/bin/sleep 30\n",
        )
        .expect("script");
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).expect("chmod");
        let bytes = fixture(root.path(), "mp4", "0.417", false);
        let extractor = Arc::new(
            FfmpegVideoExtractor::new(VideoExtractConfig {
                ffmpeg: script,
                temporary_root: root.path().to_owned(),
                ..Default::default()
            })
            .expect("extractor"),
        );
        let task = tokio::spawn({
            let extractor = extractor.clone();
            async move { extractor.extract(&bytes).await }
        });
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        let pid = loop {
            // The shell creates the file before writing it; an empty read is not a pid yet.
            if let Some(pid) = jobs(root.path()).iter().find_map(|job| {
                std::fs::read_to_string(job.join("child.pid"))
                    .ok()?
                    .parse::<u32>()
                    .ok()
            }) {
                break pid;
            }
            assert!(tokio::time::Instant::now() < deadline, "decoder must start");
            tokio::time::sleep(Duration::from_millis(10)).await;
        };
        task.abort();
        let _ = task.await;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        while !jobs(root.path()).is_empty() {
            assert!(
                tokio::time::Instant::now() < deadline,
                "cancel cleanup must complete"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(
            !Path::new(&format!("/proc/{pid}")).exists(),
            "child must be reaped"
        );
    }

    fn hold_root_lock(root: &Path, hold: Duration) -> std::thread::JoinHandle<()> {
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join(".kukuri-video-root-lock"))
            .expect("root lock file");
        // Children forked by parallel tests can briefly keep the extractor's closed lock.
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while let Err(error) = lock.try_lock() {
            assert!(
                std::time::Instant::now() < deadline,
                "external maintenance holder: {error:?}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        std::thread::spawn(move || {
            std::thread::sleep(hold);
            drop(lock);
        })
    }

    #[tokio::test]
    async fn short_scratch_maintenance_is_waited_for() {
        let root = private_tmpfs();
        let extractor = FfmpegVideoExtractor::new(VideoExtractConfig {
            temporary_root: root.path().to_owned(),
            ..Default::default()
        })
        .expect("extractor");
        let holder = hold_root_lock(root.path(), Duration::from_millis(300));
        let started = std::time::Instant::now();
        let error = extractor
            .extract(b"not a video container")
            .await
            .expect_err("garbage must not decode");
        assert!(
            matches!(error, kukuri_cn_safety::ScanError::Protocol(_)),
            "waited for maintenance: {error}"
        );
        assert!(started.elapsed() >= Duration::from_millis(250));
        holder.join().expect("holder");
        let holder = hold_root_lock(root.path(), Duration::from_millis(300));
        FfmpegVideoExtractor::new(VideoExtractConfig {
            temporary_root: root.path().to_owned(),
            ..Default::default()
        })
        .expect("startup waits for maintenance");
        holder.join().expect("holder");
        assert!(jobs(root.path()).is_empty());
    }

    #[tokio::test]
    async fn stuck_scratch_maintenance_is_reported_as_busy() {
        let root = private_tmpfs();
        let extractor = FfmpegVideoExtractor::new(VideoExtractConfig {
            temporary_root: root.path().to_owned(),
            ..Default::default()
        })
        .expect("extractor");
        let holder = hold_root_lock(root.path(), Duration::from_secs(4));
        let error = extractor
            .extract(b"not a video container")
            .await
            .expect_err("maintenance never finished");
        assert_eq!(
            kukuri_cn_safety_video::classify_failure(&error),
            kukuri_cn_safety_video::DecoderFailure::ScratchBusy
        );
        holder.join().expect("holder");
        assert!(jobs(root.path()).is_empty());
    }

    /// First exec of the real decoder is slower than the probe deadline, as on a cold page cache.
    fn cold_start_wrapper(directory: &Path, delay: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let wrapper = directory.join("ffprobe-cold");
        std::fs::write(
            &wrapper,
            format!(
                "#!/bin/sh\nif [ ! -e \"$0.warm\" ]; then : > \"$0.warm\"; /bin/sleep {delay}; fi\nexec /usr/bin/ffprobe \"$@\"\n"
            ),
        )
        .expect("cold start wrapper");
        std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700))
            .expect("executable wrapper");
        wrapper
    }

    #[tokio::test]
    async fn cold_decoder_start_does_not_consume_the_probe_deadline() {
        let root = private_tmpfs();
        let programs = tempfile::tempdir().expect("trusted test wrapper directory");
        let wrapper = cold_start_wrapper(programs.path(), "2");
        let extractor = FfmpegVideoExtractor::new(VideoExtractConfig {
            ffprobe: wrapper.clone(),
            temporary_root: root.path().to_owned(),
            probe_timeout: Duration::from_secs(1),
            ..Default::default()
        })
        .expect("extractor");
        extractor
            .readiness_probe()
            .await
            .expect("cold first exec is absorbed before the bounded probe");
        // Page cache eviction after startup: the next probe is cold again.
        std::fs::remove_file(wrapper.with_extension("warm")).expect("evict");
        let bytes = fixture(root.path(), "webm", "0.417", false);
        extractor
            .extract(&bytes)
            .await
            .expect("probe timeout is retried once after warming the decoder");
        assert!(jobs(root.path()).is_empty());
    }

    #[tokio::test]
    async fn slow_probe_still_times_out_within_bounds() {
        use std::os::unix::fs::PermissionsExt;
        let root = private_tmpfs();
        let programs = tempfile::tempdir().expect("trusted test wrapper directory");
        let wrapper = programs.path().join("ffprobe-slow");
        std::fs::write(
            &wrapper,
            "#!/bin/sh\ncase \"$*\" in *-version*) ;; *) /bin/sleep 3 ;; esac\nexec /usr/bin/ffprobe \"$@\"\n",
        )
        .expect("slow probe wrapper");
        std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700))
            .expect("executable wrapper");
        let extractor = FfmpegVideoExtractor::new(VideoExtractConfig {
            ffprobe: wrapper,
            temporary_root: root.path().to_owned(),
            probe_timeout: Duration::from_secs(1),
            ..Default::default()
        })
        .expect("extractor");
        let bytes = fixture(root.path(), "mp4", "0.417", false);
        let started = std::time::Instant::now();
        let error = extractor.extract(&bytes).await.expect_err("slow probe");
        assert_eq!(
            kukuri_cn_safety_video::classify_failure(&error),
            kukuri_cn_safety_video::DecoderFailure::ProbeTimeout
        );
        // One warm-up and one retry at most: two probe deadlines plus a fast warm-up.
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "{:?}",
            started.elapsed()
        );
        assert!(jobs(root.path()).is_empty());
    }

    #[tokio::test]
    async fn decoder_warm_up_deadline_is_classified() {
        let root = private_tmpfs();
        let programs = tempfile::tempdir().expect("trusted test wrapper directory");
        let wrapper = cold_start_wrapper(programs.path(), "4");
        let extractor = FfmpegVideoExtractor::new(VideoExtractConfig {
            ffprobe: wrapper,
            temporary_root: root.path().to_owned(),
            probe_timeout: Duration::from_secs(1),
            decode_timeout: Duration::from_secs(2),
            ..Default::default()
        })
        .expect("extractor");
        let error = extractor.readiness_probe().await.expect_err("cold start");
        assert_eq!(
            kukuri_cn_safety_video::classify_failure(&error),
            kukuri_cn_safety_video::DecoderFailure::WarmUpTimeout
        );
        assert!(jobs(root.path()).is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn concurrent_process_spawns_do_not_report_scratch_busy() {
        const SPAWNERS: usize = 4;
        use std::os::unix::process::CommandExt;
        use std::sync::atomic::{AtomicBool, Ordering};
        let root = private_tmpfs();
        let extractor = FfmpegVideoExtractor::new(VideoExtractConfig {
            temporary_root: root.path().to_owned(),
            ..Default::default()
        })
        .expect("extractor");
        let stop = Arc::new(AtomicBool::new(false));
        // Forked children briefly share every open descriptor until exec, including a
        // root lock that the extractor has already closed.
        let spawners: Vec<_> = (0..SPAWNERS)
            .map(|_| {
                let stop = stop.clone();
                std::thread::spawn(move || {
                    while !stop.load(Ordering::Relaxed) {
                        let mut command = Command::new("/bin/true");
                        unsafe {
                            command.pre_exec(|| {
                                std::thread::sleep(Duration::from_millis(2));
                                Ok(())
                            });
                        }
                        let _ = command.status();
                    }
                })
            })
            .collect();
        let mut busy = 0;
        let mut first = None;
        for _ in 0..2000 {
            // A non-video signature is rejected right after the job directory is staged.
            match extractor.extract(b"not a video container").await {
                Err(kukuri_cn_safety::ScanError::Protocol(_)) => {}
                Err(error) => {
                    busy += 1;
                    first.get_or_insert(error.to_string());
                }
                Ok(_) => panic!("garbage must not decode"),
            }
        }
        stop.store(true, Ordering::Relaxed);
        for spawner in spawners {
            spawner.join().expect("spawner");
        }
        assert_eq!(busy, 0, "transient lock holders: {first:?}");
        assert!(jobs(root.path()).is_empty());
    }

    #[test]
    fn persistent_scratch_storage_is_rejected() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().expect("ordinary disk directory");
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700))
            .expect("private disk directory");
        assert!(
            FfmpegVideoExtractor::new(VideoExtractConfig {
                temporary_root: root.path().to_owned(),
                ..Default::default()
            })
            .is_err()
        );
    }
}
