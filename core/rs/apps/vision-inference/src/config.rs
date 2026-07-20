use std::env;
use std::path::PathBuf;

use tracking_core::TrackerConfig;
use vision_core::FrameSampler;

const DEFAULT_MODEL: &str = "core/yolo/models/yolo11n.onnx";
const DEFAULT_LOG: &str = "logs/vision-inference.log";
const DEFAULT_FPS: f64 = 5.0;
const DEFAULT_CONFIDENCE: f32 = 0.25;
const DEFAULT_NMS: f32 = 0.45;

#[derive(Debug)]
pub(crate) struct Options {
    pub source: String,
    pub source_id: String,
    pub model: PathBuf,
    pub classes: Option<PathBuf>,
    pub spatial_config: Option<PathBuf>,
    pub database_url: Option<String>,
    pub log_path: PathBuf,
    pub processing_fps: f64,
    pub confidence_threshold: f32,
    pub nms_threshold: f32,
    pub display: bool,
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
    let mut processing_fps = DEFAULT_FPS;
    let mut confidence_threshold = DEFAULT_CONFIDENCE;
    let mut nms_threshold = DEFAULT_NMS;
    let mut display = false;
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
        log_path,
        processing_fps,
        confidence_threshold,
        nms_threshold,
        display,
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
           --source-id ID          Identidad lógica de la fuente\n\
           --fps 5                 Frecuencia independiente de inferencia\n\
           --confidence 0.25       Confianza mínima\n\
           --nms 0.45              Umbral IoU para NMS por clase\n\
           --log RUTA              Log de detecciones normalizadas\n\
           --display               Mostrar cajas en una ventana\n\
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
