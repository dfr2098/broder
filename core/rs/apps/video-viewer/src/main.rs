mod config;
mod logger;
mod viewer;

use opencv::{
    prelude::*,
    videoio::{self, VideoCapture},
};

use config::{parse_options, print_help};
use logger::Logger;
use viewer::{log_video_info, read_video_info, run_viewer};

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
    logger.info(format!("log: {}", options.log_path.display()))?;
    logger.info(format!("abriendo video: {}", options.path.display()))?;

    let path = options.path.to_string_lossy();
    let mut capture = VideoCapture::from_file(&path, videoio::CAP_ANY)?;
    if !capture.is_opened()? {
        logger.warn(format!("no se pudo abrir el video: {path}"))?;
        return Err(opencv::Error::new(
            opencv::core::StsError,
            format!("no se pudo abrir el video: {path}"),
        )
        .into());
    }

    let info = read_video_info(&capture)?;
    log_video_info(&mut logger, &options, &info)?;
    if options.info_only {
        logger.info("consulta de metadatos finalizada")?;
        return Ok(());
    }

    run_viewer(&mut capture, &options, &info, &mut logger)
}
