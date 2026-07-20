use std::thread;
use std::time::{Duration, Instant};

use opencv::{
    core::Mat,
    highgui,
    prelude::*,
    videoio::{self, VideoCapture},
};
use spatial_core::{CameraSpatialModel, SpatialTrack};
use tracking_core::{MultiObjectTracker, TrackingUpdate, VisionTrack};
use vision_core::{FrameSampler, VisionDetection};

use crate::config::Options;
use crate::display::{DisplayContext, draw_detections};
use crate::logger::Logger;
use crate::persistence::VisionEventPublisher;
use crate::security::redact_source;
use crate::yolo::YoloEngine;

const WINDOW_NAME: &str = "Little Brother - YOLO inference";

pub(crate) fn run_stream(
    options: &Options,
    engine: &mut YoloEngine,
    spatial_model: Option<&CameraSpatialModel>,
    event_publisher: Option<&mut VisionEventPublisher>,
    logger: &mut Logger,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut capture = VideoCapture::from_file(&options.source, videoio::CAP_ANY)?;
    if !capture.is_opened()? {
        return Err(format!(
            "no se pudo abrir la fuente: {}",
            redact_source(&options.source)
        )
        .into());
    }

    let width = capture.get(videoio::CAP_PROP_FRAME_WIDTH)?.round() as i32;
    let height = capture.get(videoio::CAP_PROP_FRAME_HEIGHT)?.round() as i32;
    let source_fps = capture.get(videoio::CAP_PROP_FPS)?;
    let is_live = options.source.starts_with("rtsp://") || options.source.starts_with("rtsps://");
    logger.info(format!(
        "flujo abierto: {}x{} fps_fuente={source_fps:.3} tipo={}",
        width,
        height,
        if is_live { "RTSP" } else { "archivo" }
    ))?;

    if options.display {
        highgui::named_window(WINDOW_NAME, highgui::WINDOW_NORMAL)?;
        highgui::resize_window(WINDOW_NAME, width.min(1280), height.min(960))?;
    }

    let stream_started = Instant::now();
    let mut sampler = FrameSampler::new(options.processing_fps)?;
    let mut tracker = MultiObjectTracker::new(&options.source_id, options.tracker_config)?;
    let mut frame = Mat::default();
    let mut captured_frames = 0_u64;
    let mut inference_count = 0_u64;
    let mut detection_count = 0_u64;
    let mut finished_track_count = 0_u64;
    let mut total_inference_ms = 0.0_f64;
    let mut maximum_inference_ms = 0.0_f64;
    let mut event_publisher = event_publisher;
    let mut last_persistence_metrics = Instant::now();

    loop {
        if !capture.read(&mut frame)? || frame.empty() {
            logger.info("fin del flujo")?;
            break;
        }
        captured_frames += 1;
        let frame_id = capture
            .get(videoio::CAP_PROP_POS_FRAMES)?
            .round()
            .max(captured_frames as f64) as u64;
        let timestamp_ms = source_timestamp_ms(
            &capture,
            is_live,
            captured_frames,
            source_fps,
            stream_started,
        )?;
        if !sampler.should_process(timestamp_ms) {
            continue;
        }

        let inference_started = Instant::now();
        let candidates = engine.infer(&frame)?;
        let inference_ms = inference_started.elapsed().as_secs_f64() * 1_000.0;
        let detections = candidates
            .into_iter()
            .enumerate()
            .map(|(sequence, candidate)| {
                VisionDetection::from_candidate(
                    &options.source_id,
                    frame_id,
                    timestamp_ms,
                    sequence,
                    candidate,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        inference_count += 1;
        total_inference_ms += inference_ms;
        maximum_inference_ms = maximum_inference_ms.max(inference_ms);
        detection_count += detections.len() as u64;
        log_detections(logger, frame_id, timestamp_ms, inference_ms, &detections)?;
        if let Some(publisher) = event_publisher.as_deref_mut() {
            publisher.publish_all(&detections)?;
            if last_persistence_metrics.elapsed() >= Duration::from_secs(10) {
                let metrics = publisher.snapshot();
                logger.info(format!(
                    "PERSISTENCE connected={} queued={} persisted={} dropped={}",
                    metrics.connected, metrics.queued, metrics.persisted, metrics.dropped
                ))?;
                last_persistence_metrics = Instant::now();
            }
        }
        let tracking = tracker.process_frame(frame_id, timestamp_ms, &detections)?;
        finished_track_count += tracking.finished_tracks.len() as u64;
        log_tracking_update(logger, &tracking, tracker.active_tracks())?;
        let spatial_tracks =
            locate_updated_tracks(spatial_model, &tracking, tracker.active_tracks())?;
        log_spatial_tracks(logger, &spatial_tracks)?;

        if options.display {
            draw_detections(
                &mut frame,
                &DisplayContext {
                    detections: &detections,
                    assignments: &tracking.assignments,
                    spatial_tracks: &spatial_tracks,
                    spatial_model,
                    active_track_count: tracker.active_tracks().len(),
                    inference_ms,
                    processing_fps: options.processing_fps,
                },
            )?;
            highgui::imshow(WINDOW_NAME, &frame)?;
            let delay_ms = remaining_cycle_time(inference_started, options.processing_fps, is_live)
                .as_millis()
                .max(1) as i32;
            let key = highgui::wait_key(delay_ms)? & 0xff;
            if matches!(key, 27 | 81 | 113) {
                logger.info("cierre solicitado por el usuario")?;
                break;
            }
        } else if !is_live {
            thread::sleep(remaining_cycle_time(
                inference_started,
                options.processing_fps,
                false,
            ));
        }

        if options
            .max_inferences
            .is_some_and(|maximum| inference_count >= maximum)
        {
            logger.info(format!("límite de {inference_count} inferencias alcanzado"))?;
            break;
        }
    }

    if options.display {
        highgui::destroy_window(WINDOW_NAME)?;
    }
    let remaining_tracks = tracker.finish_all();
    finished_track_count += remaining_tracks.len() as u64;
    for track in &remaining_tracks {
        log_finished_track(logger, track)?;
    }
    let persistence = if let Some(publisher) = event_publisher {
        publisher.finish()?
    } else {
        Default::default()
    };
    logger.info(format!(
        "PERSISTENCE_FINAL connected={} queued={} persisted={} dropped={}",
        persistence.connected, persistence.queued, persistence.persisted, persistence.dropped
    ))?;
    let elapsed_seconds = stream_started.elapsed().as_secs_f64();
    let average_inference_ms = if inference_count == 0 {
        0.0
    } else {
        total_inference_ms / inference_count as f64
    };
    let effective_fps = if elapsed_seconds > 0.0 {
        inference_count as f64 / elapsed_seconds
    } else {
        0.0
    };
    logger.info(format!(
        "METRICS elapsed_s={elapsed_seconds:.3} effective_inference_fps={effective_fps:.3} inference_ms_avg={average_inference_ms:.3} inference_ms_max={maximum_inference_ms:.3}"
    ))?;
    logger.info(format!(
        "resumen: capturados={captured_frames} inferencias={inference_count} detecciones={detection_count} detecciones_persistidas={} detecciones_descartadas={} tracks_finalizados={finished_track_count}",
        persistence.persisted,
        persistence.dropped
    ))?;
    Ok(())
}

fn locate_updated_tracks(
    spatial_model: Option<&CameraSpatialModel>,
    update: &TrackingUpdate,
    active_tracks: &[VisionTrack],
) -> Result<Vec<SpatialTrack>, Box<dyn std::error::Error>> {
    let Some(model) = spatial_model else {
        return Ok(Vec::new());
    };
    update
        .assignments
        .iter()
        .filter_map(|assignment| {
            active_tracks
                .iter()
                .find(|track| track.track_id == assignment.track_id)
        })
        .map(|track| model.locate(track).map_err(Into::into))
        .collect()
}

fn remaining_cycle_time(started: Instant, processing_fps: f64, is_live: bool) -> Duration {
    if is_live {
        return Duration::from_millis(1);
    }
    Duration::from_secs_f64(1.0 / processing_fps).saturating_sub(started.elapsed())
}

fn source_timestamp_ms(
    capture: &VideoCapture,
    is_live: bool,
    captured_frames: u64,
    source_fps: f64,
    started: Instant,
) -> opencv::Result<u64> {
    if is_live {
        return Ok(started.elapsed().as_millis() as u64);
    }

    let media_time = capture.get(videoio::CAP_PROP_POS_MSEC)?;
    if media_time.is_finite() && media_time > 0.0 {
        Ok(media_time.round() as u64)
    } else if source_fps.is_finite() && source_fps > 0.0 {
        Ok((((captured_frames - 1) as f64 / source_fps) * 1_000.0).round() as u64)
    } else {
        Ok(started.elapsed().as_millis() as u64)
    }
}

fn log_detections(
    logger: &mut Logger,
    frame_id: u64,
    timestamp_ms: u64,
    inference_ms: f64,
    detections: &[VisionDetection],
) -> std::io::Result<()> {
    logger.info(format!(
        "frame={frame_id} timestamp_ms={timestamp_ms} inferencia_ms={inference_ms:.2} detecciones={}",
        detections.len()
    ))?;
    for detection in detections {
        let bounding_box = detection.bounding_box;
        logger.info(format!(
            "DETECTION id={} frame={} timestamp_ms={} class_id={} class={} confidence={:.4} bbox_norm=[{:.6},{:.6},{:.6},{:.6}]",
            detection.detection_id,
            detection.frame_id,
            detection.timestamp_ms,
            detection.class_id,
            detection.class_name,
            detection.confidence,
            bounding_box.x,
            bounding_box.y,
            bounding_box.width,
            bounding_box.height
        ))?;
    }
    Ok(())
}

fn log_tracking_update(
    logger: &mut Logger,
    update: &TrackingUpdate,
    active_tracks: &[VisionTrack],
) -> std::io::Result<()> {
    logger.info(format!(
        "tracking: activos={} finalizados={} asignaciones={}",
        active_tracks.len(),
        update.finished_tracks.len(),
        update.assignments.len()
    ))?;
    for track in active_tracks {
        let latest = track.latest_observation();
        let bounding_box = latest.bounding_box;
        logger.info(format!(
            "TRACK id={} camera={} state={} class_id={} class={} observations={} missed={} confidence_avg={:.4} last_timestamp_ms={} bbox_norm=[{:.6},{:.6},{:.6},{:.6}]",
            track.track_id,
            track.camera_id,
            track.state,
            track.class_id,
            track.class_name,
            track.observation_count(),
            track.missed_frames,
            track.accumulated_confidence,
            track.last_observed_at_ms,
            bounding_box.x,
            bounding_box.y,
            bounding_box.width,
            bounding_box.height
        ))?;
    }
    for track in &update.finished_tracks {
        log_finished_track(logger, track)?;
    }
    Ok(())
}

fn log_finished_track(logger: &mut Logger, track: &VisionTrack) -> std::io::Result<()> {
    logger.info(format!(
        "TRACK_FINISHED id={} camera={} class_id={} class={} observations={} started_at_ms={} last_observed_at_ms={} confidence_avg={:.4}",
        track.track_id,
        track.camera_id,
        track.class_id,
        track.class_name,
        track.observation_count(),
        track.started_at_ms,
        track.last_observed_at_ms,
        track.accumulated_confidence
    ))
}

fn log_spatial_tracks(logger: &mut Logger, tracks: &[SpatialTrack]) -> std::io::Result<()> {
    for track in tracks {
        let zones = track
            .occupied_zones
            .iter()
            .map(|zone| zone.zone_id.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let crossings = track
            .crossed_lines
            .iter()
            .map(|line| line.line_id.as_str())
            .collect::<Vec<_>>()
            .join(",");
        logger.info(format!(
            "SPATIAL track={} timestamp_ms={} anchor=[{:.6},{:.6}] inside={} zones=[{}] crossings=[{}]",
            track.track_id,
            track.timestamp_ms,
            track.anchor.x,
            track.anchor.y,
            track.inside_observation_region,
            zones,
            crossings
        ))?;
    }
    Ok(())
}
