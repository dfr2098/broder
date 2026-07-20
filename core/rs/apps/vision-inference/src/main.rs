mod classes;
mod config;
mod display;
mod logger;
mod persistence;
mod spatial_config;
mod stream;
mod yolo;

use std::time::Instant;

use classes::load_class_names;
use config::{parse_options, print_help};
use logger::Logger;
use persistence::VisionEventPublisher;
use spatial_config::load_spatial_model;
use stream::run_stream;
use yolo::YoloEngine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = match parse_options() {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            print_help();
            std::process::exit(2);
        }
    };

    let mut logger = Logger::open(&options.log_path)?;
    logger.info(format!(
        "fuente={} id={}",
        options.source, options.source_id
    ))?;

    let spatial_model = options
        .spatial_config
        .as_deref()
        .map(load_spatial_model)
        .transpose()?;
    if let Some(model) = &spatial_model {
        if model.camera_id != options.source_id {
            return Err(format!(
                "la configuración espacial es para {}, pero --source-id es {}",
                model.camera_id, options.source_id
            )
            .into());
        }
        logger.info(format!(
            "modelo espacial cargado: zonas={} líneas={}",
            model.zones.len(),
            model.lines.len()
        ))?;
    }
    logger.info(format!(
        "modelo={} fps={:.2} confianza={:.2} nms={:.2}",
        options.model.display(),
        options.processing_fps,
        options.confidence_threshold,
        options.nms_threshold
    ))?;

    let mut event_publisher = options
        .database_url
        .as_deref()
        .map(|database_url| VisionEventPublisher::connect(database_url, &options.source_id))
        .transpose()?;
    if event_publisher.is_some() {
        logger.info("persistencia PostgreSQL activa: temporal.vision_detection")?;
    } else {
        logger.info("persistencia desactivada: DATABASE_URL no configurada")?;
    }

    let class_names = load_class_names(options.classes.as_deref())?;
    let model_started = Instant::now();
    let mut engine = YoloEngine::load(
        &options.model,
        class_names,
        options.confidence_threshold,
        options.nms_threshold,
    )?;
    logger.info(format!(
        "modelo cargado con OpenCV DNN/CPU en {:.1} ms",
        model_started.elapsed().as_secs_f64() * 1_000.0
    ))?;

    run_stream(
        &options,
        &mut engine,
        spatial_model.as_ref(),
        event_publisher.as_mut(),
        &mut logger,
    )
}
