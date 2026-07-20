use std::env;
use std::fmt::{self, Display};
use std::net::SocketAddr;
use std::path::PathBuf;

use tracking_core::TrackerConfig;
use vision_core::FrameSampler;

const DEFAULT_MODEL: &str = "core/yolo/models/yolo11n.onnx";
const DEFAULT_LOG: &str = "logs/vision-inference.log";
const DEFAULT_FPS: f64 = 5.0;
const DEFAULT_CONFIDENCE: f32 = 0.25;
const DEFAULT_NMS: f32 = 0.45;
const DEFAULT_PERSISTENCE_QUEUE: usize = 256;
const DEFAULT_PERSISTENCE_BATCH: usize = 25;
const DEFAULT_PERSISTENCE_FLUSH_MS: u64 = 500;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PersistenceMode {
    Required,
    BestEffort,
}

impl Display for PersistenceMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Required => "required",
            Self::BestEffort => "best-effort",
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PersistenceConfig {
    pub mode: PersistenceMode,
    pub queue_capacity: usize,
    pub batch_size: usize,
    pub flush_interval_ms: u64,
}

#[derive(Debug)]
pub(crate) struct Options {
    pub source: String,
    pub source_id: String,
    pub model: PathBuf,
    pub classes: Option<PathBuf>,
    pub spatial_config: Option<PathBuf>,
    pub database_url: Option<String>,
    pub persistence: PersistenceConfig,
    pub log_path: PathBuf,
    pub processing_fps: f64,
    pub confidence_threshold: f32,
    pub nms_threshold: f32,
    pub display: bool,
    pub loop_video: bool,
    pub web_bind: Option<SocketAddr>,
    pub max_inferences: Option<u64>,
    pub tracker_config: TrackerConfig,
}

pub(crate) fn parse_options() -> Result<Options, String> {
    let mut source = None;
    let mut source_id = "camera-1".to_owned();
    let mut model = PathBuf::from(DEFAULT_MODEL);
    let mut classes = None;
    let mut spatial_config = None;
    let mut database_url = env::var("DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let mut log_path = PathBuf::from(DEFAULT_LOG);
    let mut persistence = PersistenceConfig {
        mode: PersistenceMode::Required,
        queue_capacity: DEFAULT_PERSISTENCE_QUEUE,
        batch_size: DEFAULT_PERSISTENCE_BATCH,
        flush_interval_ms: DEFAULT_PERSISTENCE_FLUSH_MS,
    };
    let mut processing_fps = DEFAULT_FPS;
    let mut confidence_threshold = DEFAULT_CONFIDENCE;
    let mut nms_threshold = DEFAULT_NMS;
    let mut display = false;
    let mut loop_video = false;
    let mut web_bind = None;
    let mut max_inferences = None;
    let mut tracker_config = TrackerConfig::default();
    let mut args = env::args().skip(1);

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--source-id" => source_id = next_value(&mut args, "--source-id")?,
            "--model" => model = PathBuf::from(next_value(&mut args, "--model")?),
            "--classes" => classes = Some(PathBuf::from(next_value(&mut args, "--classes")?)),
            "--spatial-config" => {
                spatial_config = Some(PathBuf::from(next_value(&mut args, "--spatial-config")?));
            }
            "--database-url" => {
                database_url = Some(next_value(&mut args, "--database-url")?);
            }
            "--no-persistence" => database_url = None,
            "--persistence-mode" => {
                let value = next_value(&mut args, "--persistence-mode")?;
                persistence.mode = match value.as_str() {
                    "required" => PersistenceMode::Required,
                    "best-effort" => PersistenceMode::BestEffort,
                    _ => return Err("--persistence-mode debe ser required o best-effort".into()),
                };
            }
            "--persistence-queue" => {
                persistence.queue_capacity = next_value(&mut args, "--persistence-queue")?
                    .parse::<usize>()
                    .map_err(|_| "--persistence-queue debe ser un entero".to_owned())?;
            }
            "--persistence-batch" => {
                persistence.batch_size = next_value(&mut args, "--persistence-batch")?
                    .parse::<usize>()
                    .map_err(|_| "--persistence-batch debe ser un entero".to_owned())?;
            }
            "--persistence-flush-ms" => {
                persistence.flush_interval_ms = next_value(&mut args, "--persistence-flush-ms")?
                    .parse::<u64>()
                    .map_err(|_| "--persistence-flush-ms debe ser un entero".to_owned())?;
            }
            "--log" => log_path = PathBuf::from(next_value(&mut args, "--log")?),
            "--fps" => {
                let value = next_value(&mut args, "--fps")?;
                processing_fps = value
                    .parse::<f64>()
                    .map_err(|_| format!("FPS inválidos: {value}"))?;
            }
            "--confidence" => {
                let value = next_value(&mut args, "--confidence")?;
                confidence_threshold = value
                    .parse::<f32>()
                    .map_err(|_| format!("confianza inválida: {value}"))?;
            }
            "--nms" => {
                let value = next_value(&mut args, "--nms")?;
                nms_threshold = value
                    .parse::<f32>()
                    .map_err(|_| format!("umbral NMS inválido: {value}"))?;
            }
            "--max-inferences" => {
                let value = next_value(&mut args, "--max-inferences")?;
                max_inferences = Some(
                    value
                        .parse::<u64>()
                        .map_err(|_| format!("máximo de inferencias inválido: {value}"))?,
                );
            }
            "--track-min-hits" => {
                tracker_config.minimum_confirmed_hits = next_value(&mut args, "--track-min-hits")?
                    .parse::<u32>()
                    .map_err(|_| "--track-min-hits debe ser un entero".to_owned())?;
            }
            "--track-max-missed" => {
                tracker_config.maximum_missed_frames = next_value(&mut args, "--track-max-missed")?
                    .parse::<u32>()
                    .map_err(|_| "--track-max-missed debe ser un entero".to_owned())?;
            }
            "--track-max-lost-ms" => {
                tracker_config.maximum_lost_ms = next_value(&mut args, "--track-max-lost-ms")?
                    .parse::<u64>()
                    .map_err(|_| "--track-max-lost-ms debe ser un entero".to_owned())?;
            }
            "--track-min-iou" => {
                tracker_config.minimum_iou = next_value(&mut args, "--track-min-iou")?
                    .parse::<f32>()
                    .map_err(|_| "--track-min-iou debe ser numérico".to_owned())?;
            }
            "--track-max-distance" => {
                tracker_config.maximum_center_distance =
                    next_value(&mut args, "--track-max-distance")?
                        .parse::<f32>()
                        .map_err(|_| "--track-max-distance debe ser numérico".to_owned())?;
            }
            "--display" => display = true,
            "--loop-video" => loop_video = true,
            "--web-bind" => {
                let value = next_value(&mut args, "--web-bind")?;
                web_bind = Some(
                    value
                        .parse::<SocketAddr>()
                        .map_err(|_| format!("dirección web inválida: {value}"))?,
                );
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            value if value.starts_with('-') => return Err(format!("opción desconocida: {value}")),
            value => {
                if source.replace(value.to_owned()).is_some() {
                    return Err("solo se admite una fuente de video".to_owned());
                }
            }
        }
    }

    FrameSampler::new(processing_fps).map_err(str::to_owned)?;
    if source_id.trim().is_empty() {
        return Err("--source-id no puede estar vacío".to_owned());
    }
    validate_unit_interval(confidence_threshold, "--confidence")?;
    validate_unit_interval(nms_threshold, "--nms")?;
    if max_inferences == Some(0) {
        return Err("--max-inferences debe ser mayor que cero".to_owned());
    }
    if persistence.queue_capacity == 0
        || persistence.batch_size == 0
        || persistence.flush_interval_ms == 0
    {
        return Err(
            "la cola, el lote y el intervalo de persistencia deben ser mayores que cero".to_owned(),
        );
    }
    tracker_config = tracker_config
        .validate()
        .map_err(|error| error.to_string())?;

    Ok(Options {
        source: source.ok_or_else(|| "falta el archivo de video o URL RTSP".to_owned())?,
        source_id,
        model,
        classes,
        spatial_config,
        database_url,
        persistence,
        log_path,
        processing_fps,
        confidence_threshold,
        nms_threshold,
        display,
        loop_video,
        web_bind,
        max_inferences,
        tracker_config,
    })
}

fn next_value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("falta el valor después de {option}"))
}

fn validate_unit_interval(value: f32, option: &str) -> Result<(), String> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(format!("{option} debe estar entre 0 y 1"))
    }
}

pub(crate) fn print_help() {
    println!(
        "Uso: vision-inference [opciones] <archivo|rtsp://...>\n\
         \n\
         Opciones:\n\
           --model RUTA            Modelo YOLO 11 ONNX\n\
           --classes RUTA          Una clase por línea (predeterminado: COCO)\n\
           --spatial-config RUTA   Geometría normalizada de la cámara\n\
           --database-url URL      PostgreSQL (o variable DATABASE_URL)\n\
           --no-persistence        Ejecutar sin guardar detecciones\n\
           --persistence-mode MODO required | best-effort\n\
           --persistence-queue N   Eventos máximos en espera (256)\n\
           --persistence-batch N   Detecciones por transacción (25)\n\
           --persistence-flush-ms N  Flush máximo en milisegundos (500)\n\
           --source-id ID          Identidad lógica de la fuente\n\
           --fps 5                 Frecuencia independiente de inferencia\n\
           --confidence 0.25       Confianza mínima\n\
           --nms 0.45              Umbral IoU para NMS por clase\n\
           --log RUTA              Log de detecciones normalizadas\n\
           --display               Mostrar cajas en una ventana\n\
           --loop-video            Reiniciar un archivo al llegar al final\n\
           --web-bind IP:PUERTO    Panel HTTP/WebSocket opcional\n\
           --max-inferences N      Detener una prueba después de N inferencias\n\
           --track-min-hits 2      Observaciones para confirmar un track\n\
           --track-max-missed 5   Pérdidas consecutivas toleradas\n\
           --track-max-lost-ms 1500  Tiempo máximo sin observación\n\
           --track-min-iou 0.05   IoU mínimo de asociación\n\
           --track-max-distance 0.25  Distancia normalizada máxima\n\
         \n\
         En la ventana: Q o Esc cierra el proceso."
    );
}
