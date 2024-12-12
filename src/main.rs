use anyhow::{Context, Result};
use std::process::{Command, Stdio};
use std::io::{BufReader, BufRead}; 
use std::thread;
use std::time::Instant;
use std::path::Path;
use rayon::ThreadPoolBuilder;
use num_cpus;

fn main() -> Result<()> {
    convert_mp4_to_gif("./data/QwerGPT-UI.mp4", "./data/QwerGPT-UI.gif", 10)?;
    Ok(())
}

fn convert_mp4_to_gif(input_path: &str, output_path: &str, fps: u32) -> Result<()> {
    let num_cores = num_cpus::get();
    ThreadPoolBuilder::new()
        .num_threads(num_cores)
        .build_global()
        .unwrap();

    println!("Using {} CPU cores for parallel processing", num_cores);
    println!("Starting conversion...");
    
    let temp_dir = tempfile::tempdir()?;
    let frames_path = temp_dir.path().join("frame%d.png");
    
    println!("Extracting frames with ffmpeg...");
    
    // Extract frames using ffmpeg
    let mut child = Command::new("ffmpeg")
        .args([
            "-i", input_path,
            "-vf", &format!("fps={},scale=1280:-1", fps),
            "-frame_pts", "1",
            frames_path.to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to execute ffmpeg")?;

    let stderr = child.stderr.take().unwrap();
    let stderr_handle = thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                println!("ffmpeg: {}", line);
            }
        }
    });

    let status = child.wait().context("Failed to wait for ffmpeg")?;
    stderr_handle.join().unwrap();

    if !status.success() {
        anyhow::bail!("FFmpeg process failed");
    }

    println!("Frame extraction completed. Processing frames with gifski...");

    let start_time = Instant::now();

    // Calculate total frames and collect frame paths
    let mut frame_paths = Vec::new();
    let mut frame_count = 0;
    while let Some(frame_path) = temp_dir.path()
        .join(format!("frame{}.png", frame_count + 1))
        .to_str() {
        if !Path::new(frame_path).exists() {
            break;
        }
        frame_paths.push(frame_path.to_string());
        frame_count += 1;
    }

    if frame_count == 0 {
        anyhow::bail!("No frames were extracted");
    }

    println!("Total frames to process: {}", frame_count);
    println!("Converting frames to GIF using gifski...");

    // Build gifski command with frame paths as arguments
    let mut cmd = Command::new("gifski");
    cmd.args([
        "--quality", "100",
        "--fps", &fps.to_string(),
        "--width", "1280",
        "-o", output_path
    ]);
    
    // Add all frame paths as arguments
    cmd.args(&frame_paths);

    // Execute gifski
    let status = cmd.status()
        .context("Failed to execute gifski")?;

    if !status.success() {
        anyhow::bail!("Gifski process failed");
    }

    let total_time = start_time.elapsed();
    println!(
        "GIF creation completed! Processed {} frames in {:.2}s",
        frame_count,
        total_time.as_secs_f32()
    );

    Ok(())
}
