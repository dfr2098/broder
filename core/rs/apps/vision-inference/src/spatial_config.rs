use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

use spatial_core::{
    CameraSpatialModel, LineRole, NormalizedPoint, SpatialPolygon, SpatialZone, VirtualLine,
    ZoneKind,
};

pub(crate) fn load_spatial_model(
    path: &Path,
) -> Result<CameraSpatialModel, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut camera_id = None;
    let mut observation_region = None;
    let mut zones = Vec::new();
    let mut lines = Vec::new();

    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line_number = index + 1;
        let line = line?;
        let content = line.trim();
        if content.is_empty() || content.starts_with('#') {
            continue;
        }
        let (key, value) = content
            .split_once('=')
            .ok_or_else(|| invalid(line_number, "se esperaba clave=valor"))?;
        match key.trim() {
            "camera" => camera_id = Some(value.trim().to_owned()),
            "observation" => observation_region = Some(parse_polygon(value, line_number)?),
            "zone" => zones.push(parse_zone(value, line_number)?),
            "line" => lines.push(parse_line(value, line_number)?),
            unknown => {
                return Err(invalid(line_number, format!("clave desconocida: {unknown}")).into());
            }
        }
    }

    Ok(CameraSpatialModel::new(
        camera_id.ok_or_else(|| invalid(0, "falta camera"))?,
        observation_region.ok_or_else(|| invalid(0, "falta observation"))?,
        zones,
        lines,
    )?)
}

fn parse_zone(value: &str, line: usize) -> Result<SpatialZone, Box<dyn std::error::Error>> {
    let fields = value.split('|').map(str::trim).collect::<Vec<_>>();
    if fields.len() != 6 {
        return Err(invalid(
            line,
            "zone requiere id|nombre|tipo|parent_id|dirección|polígono",
        )
        .into());
    }
    let kind = match fields[2] {
        "conveyor" => ZoneKind::Conveyor,
        "lane" => ZoneKind::Lane,
        "entry" => ZoneKind::Entry,
        "exit" => ZoneKind::Exit,
        value if value.starts_with("custom:") => {
            ZoneKind::Custom(value.trim_start_matches("custom:").to_owned())
        }
        value => return Err(invalid(line, format!("tipo de zona desconocido: {value}")).into()),
    };
    Ok(SpatialZone::new(
        fields[0],
        fields[1],
        kind,
        optional(fields[3]),
        optional(fields[4]),
        parse_polygon(fields[5], line)?,
    )?)
}

fn parse_line(value: &str, line: usize) -> Result<VirtualLine, Box<dyn std::error::Error>> {
    let fields = value.split('|').map(str::trim).collect::<Vec<_>>();
    if fields.len() != 5 {
        return Err(invalid(line, "line requiere id|nombre|rol|inicio|fin").into());
    }
    let role = match fields[2] {
        "entry" => LineRole::Entry,
        "exit" => LineRole::Exit,
        "boundary" => LineRole::Boundary,
        value => return Err(invalid(line, format!("rol de línea desconocido: {value}")).into()),
    };
    Ok(VirtualLine::new(
        fields[0],
        fields[1],
        role,
        parse_point(fields[3], line)?,
        parse_point(fields[4], line)?,
    )?)
}

fn parse_polygon(value: &str, line: usize) -> Result<SpatialPolygon, Box<dyn std::error::Error>> {
    let points = value
        .split(';')
        .map(|value| parse_point(value.trim(), line))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SpatialPolygon::new(points)?)
}

fn parse_point(value: &str, line: usize) -> Result<NormalizedPoint, io::Error> {
    let (x, y) = value
        .split_once(',')
        .ok_or_else(|| invalid(line, format!("punto inválido: {value}")))?;
    let x = x
        .trim()
        .parse::<f32>()
        .map_err(|_| invalid(line, format!("coordenada inválida: {x}")))?;
    let y = y
        .trim()
        .parse::<f32>()
        .map_err(|_| invalid(line, format!("coordenada inválida: {y}")))?;
    NormalizedPoint::new(x, y).map_err(|error| invalid(line, error.to_string()))
}

fn optional(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

fn invalid(line: usize, message: impl Into<String>) -> io::Error {
    let message = message.into();
    io::Error::new(
        io::ErrorKind::InvalidData,
        if line == 0 {
            message
        } else {
            format!("línea {line}: {message}")
        },
    )
}
