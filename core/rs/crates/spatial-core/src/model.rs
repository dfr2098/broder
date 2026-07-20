use std::collections::HashSet;
use std::fmt::{self, Display};

use crate::{NormalizedPoint, SpatialError, SpatialPolygon};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ZoneKind {
    Conveyor,
    Lane,
    Entry,
    Exit,
    Custom(String),
}

impl Display for ZoneKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conveyor => formatter.write_str("conveyor"),
            Self::Lane => formatter.write_str("lane"),
            Self::Entry => formatter.write_str("entry"),
            Self::Exit => formatter.write_str("exit"),
            Self::Custom(value) => formatter.write_str(value),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineRole {
    Entry,
    Exit,
    Boundary,
}

impl Display for LineRole {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Entry => "entry",
            Self::Exit => "exit",
            Self::Boundary => "boundary",
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpatialZone {
    pub zone_id: String,
    pub name: String,
    pub kind: ZoneKind,
    pub parent_id: Option<String>,
    pub direction: Option<String>,
    pub polygon: SpatialPolygon,
}

impl SpatialZone {
    pub fn new(
        zone_id: impl Into<String>,
        name: impl Into<String>,
        kind: ZoneKind,
        parent_id: Option<String>,
        direction: Option<String>,
        polygon: SpatialPolygon,
    ) -> Result<Self, SpatialError> {
        let zone_id = required(zone_id.into(), "zone_id")?;
        let name = required(name.into(), "nombre de zona")?;
        Ok(Self {
            zone_id,
            name,
            kind,
            parent_id: clean_optional(parent_id),
            direction: clean_optional(direction),
            polygon,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VirtualLine {
    pub line_id: String,
    pub name: String,
    pub role: LineRole,
    pub start: NormalizedPoint,
    pub end: NormalizedPoint,
}

impl VirtualLine {
    pub fn new(
        line_id: impl Into<String>,
        name: impl Into<String>,
        role: LineRole,
        start: NormalizedPoint,
        end: NormalizedPoint,
    ) -> Result<Self, SpatialError> {
        if start == end {
            return Err(SpatialError::new(
                "una línea virtual requiere dos puntos distintos",
            ));
        }
        Ok(Self {
            line_id: required(line_id.into(), "line_id")?,
            name: required(name.into(), "nombre de línea")?,
            role,
            start,
            end,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CrossingDirection {
    NegativeToPositive,
    PositiveToNegative,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LineCrossing {
    pub line_id: String,
    pub name: String,
    pub role: LineRole,
    pub direction: CrossingDirection,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ZoneMatch {
    pub zone_id: String,
    pub name: String,
    pub kind: ZoneKind,
    pub parent_id: Option<String>,
    pub direction: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpatialTrack {
    pub track_id: String,
    pub camera_id: String,
    pub timestamp_ms: u64,
    pub anchor: NormalizedPoint,
    pub inside_observation_region: bool,
    pub occupied_zones: Vec<ZoneMatch>,
    pub crossed_lines: Vec<LineCrossing>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CameraSpatialModel {
    pub camera_id: String,
    pub observation_region: SpatialPolygon,
    pub zones: Vec<SpatialZone>,
    pub lines: Vec<VirtualLine>,
}

impl CameraSpatialModel {
    pub fn new(
        camera_id: impl Into<String>,
        observation_region: SpatialPolygon,
        zones: Vec<SpatialZone>,
        lines: Vec<VirtualLine>,
    ) -> Result<Self, SpatialError> {
        let camera_id = required(camera_id.into(), "camera_id")?;
        let zone_ids = zones
            .iter()
            .map(|zone| zone.zone_id.as_str())
            .collect::<HashSet<_>>();
        if zone_ids.len() != zones.len() {
            return Err(SpatialError::new("existen zone_id duplicados"));
        }
        for zone in &zones {
            if zone
                .parent_id
                .as_deref()
                .is_some_and(|parent| !zone_ids.contains(parent))
            {
                return Err(SpatialError::new(format!(
                    "la zona {} referencia un parent_id inexistente",
                    zone.zone_id
                )));
            }
        }
        let line_ids = lines
            .iter()
            .map(|line| line.line_id.as_str())
            .collect::<HashSet<_>>();
        if line_ids.len() != lines.len() {
            return Err(SpatialError::new("existen line_id duplicados"));
        }
        Ok(Self {
            camera_id,
            observation_region,
            zones,
            lines,
        })
    }
}

fn required(value: String, field: &str) -> Result<String, SpatialError> {
    if value.trim().is_empty() {
        Err(SpatialError::new(format!("{field} no puede estar vacío")))
    } else {
        Ok(value)
    }
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value.filter(|item| !item.trim().is_empty())
}
