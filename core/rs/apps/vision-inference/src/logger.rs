use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub(crate) struct Logger {
    file: BufWriter<File>,
    started_at: Instant,
}

impl Logger {
    pub(crate) fn open(path: &Path) -> std::io::Result<Self> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        let mut logger = Self {
            file: BufWriter::new(file),
            started_at: Instant::now(),
        };
        let unix_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        writeln!(
            logger.file,
            "\n--- nueva sesión vision-inference | unix={unix_time} ---"
        )?;
        logger.file.flush()?;
        Ok(logger)
    }

    pub(crate) fn info(&mut self, message: impl AsRef<str>) -> std::io::Result<()> {
        self.write("INFO", message)
    }

    pub(crate) fn warn(&mut self, message: impl AsRef<str>) -> std::io::Result<()> {
        self.write("WARN", message)
    }

    fn write(&mut self, level: &str, message: impl AsRef<str>) -> std::io::Result<()> {
        let elapsed = self.started_at.elapsed().as_secs_f64();
        let line = format!("[+{elapsed:09.3}s] {level} {}", message.as_ref());
        println!("{line}");
        writeln!(self.file, "{line}")?;
        self.file.flush()
    }
}
