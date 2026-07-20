mod classes;
mod config;
mod display;
mod logger;
mod persistence;
mod security;
mod spatial_config;
mod stream;
mod yolo;

use std::time::Instant;

use classes::load_class_names;
use config::{parse_options, print_help};
use logger::Logger;
use persistence::{PersistenceStartup, VisionEventPublisher};
use security::redact_source;
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
        redact_source(&options.source),
        options.source_id
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

    let mut event_publisher = if let Some(database_url) = options.database_url.as_deref() {
        let (publisher, startup) =
            VisionEventPublisher::start(database_url, &options.source_id, options.persistence)?;
        match startup {
            PersistenceStartup::Connected => logger.info(format!(
                "persistencia asíncrona activa: mode={} queue={} batch={} flush_ms={}",
                options.persistence.mode,
                options.persistence.queue_capacity,
                options.persistence.batch_size,
                options.persistence.flush_interval_ms
            ))?,
            PersistenceStartup::Recovering(message) => logger.warn(format!(
                "persistencia iniciada sin conexión; se reintentará: {message}"
            ))?,
        }
        Some(publisher)
    } else {
        logger.info("persistencia desactivada: DATABASE_URL no configurada")?;
        None
    };

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
