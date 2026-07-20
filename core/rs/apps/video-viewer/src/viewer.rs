use opencv::{
    core::{Mat, Point, Scalar},
    highgui, imgproc,
    prelude::*,
    videoio::{self, VideoCapture},
};

use crate::config::Options;
use crate::logger::Logger;

const WINDOW_NAME: &str = "Little Brother - visor provisional";

#[derive(Debug)]
pub(crate) struct VideoInfo {
    width: i32,
    height: i32,
    source_fps: f64,
    total_frames: i64,
    duration_seconds: f64,
    codec: String,
}

pub(crate) fn read_video_info(capture: &VideoCapture) -> opencv::Result<VideoInfo> {
    let width = capture.get(videoio::CAP_PROP_FRAME_WIDTH)?.round() as i32;
    let height = capture.get(videoio::CAP_PROP_FRAME_HEIGHT)?.round() as i32;
    let source_fps = capture.get(videoio::CAP_PROP_FPS)?;
    let total_frames = capture.get(videoio::CAP_PROP_FRAME_COUNT)?.round() as i64;
    let duration_seconds = if source_fps > 0.0 {
        total_frames as f64 / source_fps
    } else {
        0.0
    };
    let fourcc = capture.get(videoio::CAP_PROP_FOURCC)?.round() as u32;

    Ok(VideoInfo {
        width,
        height,
        source_fps,
        total_frames,
        duration_seconds,
        codec: fourcc_to_string(fourcc),
    })
}

fn fourcc_to_string(value: u32) -> String {
    let bytes = value.to_le_bytes();
    bytes
        .into_iter()
        .filter(|byte| byte.is_ascii_graphic())
        .map(char::from)
        .collect()
}

pub(crate) fn log_video_info(
    logger: &mut Logger,
    options: &Options,
    info: &VideoInfo,
) -> std::io::Result<()> {
    logger.info(format!("resolución: {}x{}", info.width, info.height))?;
    logger.info(format!("FPS fuente: {:.3}", info.source_fps))?;
    logger.info(format!("FPS visor: {:.3}", options.display_fps))?;
    logger.info(format!("frames: {}", info.total_frames))?;
    logger.info(format!("duración: {:.3} s", info.duration_seconds))?;
    logger.info(format!("códec: {}", info.codec))
}

pub(crate) fn run_viewer(
    capture: &mut VideoCapture,
    options: &Options,
    info: &VideoInfo,
    logger: &mut Logger,
) -> Result<(), Box<dyn std::error::Error>> {
    highgui::named_window(WINDOW_NAME, highgui::WINDOW_NORMAL)?;
    highgui::resize_window(WINDOW_NAME, info.width.min(960), info.height.min(960))?;
    logger.info("visor iniciado")?;

    let source_fps = if info.source_fps > 0.0 {
        info.source_fps
    } else {
        options.display_fps
    };
    let frame_step = (source_fps / options.display_fps).round().max(1.0) as usize;
    let delay_ms = (1_000.0 / options.display_fps).round().max(1.0) as i32;
    let mut frame = Mat::default();
    let mut paused = false;
    let mut advance_one = false;
    let mut last_progress_second = -10_i64;

    loop {
        if !paused || advance_one {
            if !capture.read(&mut frame)? || frame.empty() {
                logger.info("fin del video")?;
                break;
            }

            draw_overlay(&mut frame, capture, options, paused)?;
            highgui::imshow(WINDOW_NAME, &frame)?;

            let media_second = (capture.get(videoio::CAP_PROP_POS_MSEC)? / 1_000.0) as i64;
            if media_second >= last_progress_second + 10 {
                let frame_number = capture.get(videoio::CAP_PROP_POS_FRAMES)?.round() as i64;
                logger.info(format!(
                    "progreso: t={media_second}s/{:.0}s, frame={frame_number}/{}",
                    info.duration_seconds, info.total_frames
                ))?;
                last_progress_second = media_second;
            }

            if !paused {
                for _ in 1..frame_step {
                    if !capture.grab()? {
                        break;
                    }
                }
            }
            advance_one = false;
        }

        let key = highgui::wait_key(if paused { 30 } else { delay_ms })? & 0xff;
        match key {
            27 | 113 | 81 => {
                logger.info("cierre solicitado por el usuario")?;
                break;
            }
            32 => {
                paused = !paused;
                logger.info(if paused {
                    "video pausado"
                } else {
                    "video reanudado"
                })?;
            }
            110 | 78 if paused => {
                advance_one = true;
                logger.info("avance manual solicitado")?;
            }
            114 | 82 => {
                capture.set(videoio::CAP_PROP_POS_FRAMES, 0.0)?;
                paused = false;
                last_progress_second = -10;
                logger.info("video reiniciado")?;
            }
            _ => {}
        }
    }

    highgui::destroy_window(WINDOW_NAME)?;
    logger.info("visor cerrado")?;
    Ok(())
}

fn draw_overlay(
    frame: &mut Mat,
    capture: &VideoCapture,
    options: &Options,
    paused: bool,
) -> opencv::Result<()> {
    let frame_number = capture.get(videoio::CAP_PROP_POS_FRAMES)?.round() as i64;
    let time_seconds = capture.get(videoio::CAP_PROP_POS_MSEC)? / 1_000.0;
    let state = if paused { "PAUSA" } else { "REPRODUCIENDO" };
    let text = format!(
        "{state} | visor {:.1} FPS | t={time_seconds:.2}s | frame={frame_number}",
        options.display_fps
    );

    imgproc::put_text(
        frame,
        &text,
        Point::new(17, 32),
        imgproc::FONT_HERSHEY_SIMPLEX,
        0.62,
        Scalar::new(0.0, 0.0, 0.0, 0.0),
        4,
        imgproc::LINE_AA,
        false,
    )?;
    imgproc::put_text(
        frame,
        &text,
        Point::new(17, 32),
        imgproc::FONT_HERSHEY_SIMPLEX,
        0.62,
        Scalar::new(80.0, 255.0, 80.0, 0.0),
        1,
        imgproc::LINE_AA,
        false,
    )
}
