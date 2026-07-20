use std::env;
use std::path::PathBuf;

const DEFAULT_FPS: f64 = 5.0;

#[derive(Debug)]
pub(crate) struct Options {
    pub path: PathBuf,
    pub log_path: PathBuf,
    pub display_fps: f64,
    pub info_only: bool,
}

pub(crate) fn parse_options() -> Result<Options, String> {
    let mut args = env::args().skip(1);
    let mut path = None;
    let mut log_path = PathBuf::from("logs/video-viewer.log");
    let mut display_fps = DEFAULT_FPS;
    let mut info_only = false;

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--fps" => {
                let value = args
                    .next()
                    .ok_or_else(|| "falta el valor después de --fps".to_owned())?;
                display_fps = value
                    .parse::<f64>()
                    .map_err(|_| format!("FPS inválidos: {value}"))?;
            }
            "--log" => {
                let value = args
                    .next()
                    .ok_or_else(|| "falta la ruta después de --log".to_owned())?;
                log_path = PathBuf::from(value);
            }
            "--info" => info_only = true,
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            value if value.starts_with('-') => {
                return Err(format!("opción desconocida: {value}"));
            }
            value => {
                if path.replace(PathBuf::from(value)).is_some() {
                    return Err("solo se admite un archivo de video".to_owned());
                }
            }
        }
    }

    if !display_fps.is_finite() || display_fps <= 0.0 || display_fps > 120.0 {
        return Err("--fps debe estar entre 0 y 120".to_owned());
    }

    Ok(Options {
        path: path.ok_or_else(|| "falta la ruta del video".to_owned())?,
        log_path,
        display_fps,
        info_only,
    })
}

pub(crate) fn print_help() {
    println!(
        "Uso: video-viewer [--fps 5] [--log archivo] [--info] <video>\n\
         \n\
         Los eventos se muestran en la terminal y se anexan al archivo de log.\n\
         Valor predeterminado: logs/video-viewer.log\n\
         \n\
         Controles:\n\
           Espacio  Pausar o continuar\n\
           N        Avanzar un frame cuando está pausado\n\
           R        Volver al inicio\n\
           Q / Esc  Cerrar"
    );
}
