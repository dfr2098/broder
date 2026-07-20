use std::io;
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::{self, JoinHandle};

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use opencv::core::{Mat, Vector};
use opencv::{imgcodecs, prelude::*};
use serde::Serialize;
use serde_json::json;
use spatial_core::{CameraSpatialModel, NormalizedPoint, SpatialTrack};
use tokio::sync::{broadcast, oneshot};
use tracking_core::{TrackAssignment, VisionTrack};
use vision_core::VisionDetection;

const INDEX_HTML: &str = include_str!("../web/index.html");
const CHANNEL_CAPACITY: usize = 4;
const JPEG_QUALITY: i32 = 72;

#[derive(Clone)]
struct AppState {
    frames: broadcast::Sender<String>,
    clients: Arc<AtomicUsize>,
    client_shutdown: broadcast::Sender<()>,
}

pub(crate) struct WebDashboard {
    local_addr: SocketAddr,
    frames: broadcast::Sender<String>,
    clients: Arc<AtomicUsize>,
    client_shutdown: broadcast::Sender<()>,
    shutdown: Option<oneshot::Sender<()>>,
    worker: Option<JoinHandle<Result<(), String>>>,
}

impl WebDashboard {
    pub(crate) fn start(bind_addr: SocketAddr) -> Result<Self, Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(bind_addr)?;
        listener.set_nonblocking(true)?;
        let local_addr = listener.local_addr()?;
        let (frames, _) = broadcast::channel(CHANNEL_CAPACITY);
        let (client_shutdown, _) = broadcast::channel(1);
        let clients = Arc::new(AtomicUsize::new(0));
        let state = AppState {
            frames: frames.clone(),
            clients: clients.clone(),
            client_shutdown: client_shutdown.clone(),
        };
        let (shutdown, shutdown_receiver) = oneshot::channel();
        let (startup_sender, startup_receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("vision-web".to_owned())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|error| error.to_string())?;
                runtime.block_on(async move {
                    let listener = tokio::net::TcpListener::from_std(listener)
                        .map_err(|error| error.to_string())?;
                    let app = Router::new()
                        .route("/", get(index))
                        .route("/health", get(health))
                        .route("/ws", get(websocket_upgrade))
                        .with_state(state);
                    let _ = startup_sender.send(());
                    axum::serve(listener, app)
                        .with_graceful_shutdown(async {
                            let _ = shutdown_receiver.await;
                        })
                        .await
                        .map_err(|error| error.to_string())
                })
            })?;
        startup_receiver
            .recv()
            .map_err(|_| io::Error::other("el servidor web terminó durante la inicialización"))?;
        Ok(Self {
            local_addr,
            frames,
            clients,
            client_shutdown,
            shutdown: Some(shutdown),
            worker: Some(worker),
        })
    }

    pub(crate) fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn publish_frame(
        &self,
        frame: &Mat,
        source_id: &str,
        frame_id: u64,
        timestamp_ms: u64,
        inference_ms: f64,
        processing_fps: f64,
        detections: &[VisionDetection],
        assignments: &[TrackAssignment],
        active_tracks: &[VisionTrack],
        spatial_tracks: &[SpatialTrack],
        spatial_model: Option<&CameraSpatialModel>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.frames.receiver_count() == 0 {
            return Ok(());
        }
        let mut encoded = Vector::<u8>::new();
        let parameters = Vector::from_slice(&[imgcodecs::IMWRITE_JPEG_QUALITY, JPEG_QUALITY]);
        imgcodecs::imencode(".jpg", frame, &mut encoded, &parameters)?;
        let image = BASE64_STANDARD.encode(encoded.as_slice());
        let message = DashboardFrame {
            message_type: "frame",
            source_id,
            frame_id,
            timestamp_ms,
            width: frame.cols(),
            height: frame.rows(),
            inference_ms,
            processing_fps,
            browser_clients: self.clients.load(Ordering::Relaxed),
            image: format!("data:image/jpeg;base64,{image}"),
            detections: detections
                .iter()
                .map(|detection| {
                    detection_view(detection, assignments, active_tracks, spatial_tracks)
                })
                .collect(),
            active_tracks: active_tracks
                .iter()
                .map(|track| TrackView {
                    track_id: track.track_id.clone(),
                    class_name: track.class_name.clone(),
                    state: track.state.to_string(),
                    observations: track.observation_count(),
                    missed_frames: track.missed_frames,
                })
                .collect(),
            geometry: spatial_model.map(geometry_view),
        };
        let serialized = serde_json::to_string(&message)?;
        let _ = self.frames.send(serialized);
        Ok(())
    }
}

impl Drop for WebDashboard {
    fn drop(&mut self) {
        let _ = self.client_shutdown.send(());
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "websocket_clients": state.clients.load(Ordering::Relaxed)
    }))
}

async fn websocket_upgrade(
    websocket: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    websocket.on_upgrade(move |socket| websocket_client(socket, state))
}

async fn websocket_client(mut socket: WebSocket, state: AppState) {
    state.clients.fetch_add(1, Ordering::Relaxed);
    let mut frames = state.frames.subscribe();
    let mut shutdown = state.client_shutdown.subscribe();
    loop {
        tokio::select! {
            _ = shutdown.recv() => break,
            frame = frames.recv() => {
                match frame {
                    Ok(frame) => {
                        if socket.send(Message::Text(frame.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            message = socket.recv() => {
                if message.is_none() || matches!(message, Some(Ok(Message::Close(_))) | Some(Err(_))) {
                    break;
                }
            }
        }
    }
    state.clients.fetch_sub(1, Ordering::Relaxed);
}

fn detection_view(
    detection: &VisionDetection,
    assignments: &[TrackAssignment],
    active_tracks: &[VisionTrack],
    spatial_tracks: &[SpatialTrack],
) -> DetectionView {
    let track_id = assignments
        .iter()
        .find(|assignment| assignment.detection_id == detection.detection_id)
        .map(|assignment| assignment.track_id.as_str());
    let track_state = track_id.and_then(|track_id| {
        active_tracks
            .iter()
            .find(|track| track.track_id == track_id)
            .map(|track| track.state.to_string())
    });
    let zones = track_id
        .and_then(|track_id| {
            spatial_tracks
                .iter()
                .find(|track| track.track_id == track_id)
        })
        .map(|track| {
            track
                .occupied_zones
                .iter()
                .map(|zone| zone.name.clone())
                .collect()
        })
        .unwrap_or_default();
    let bounding_box = detection.bounding_box;
    DetectionView {
        detection_id: detection.detection_id.clone(),
        class_name: detection.class_name.clone(),
        confidence: detection.confidence,
        x: bounding_box.x,
        y: bounding_box.y,
        width: bounding_box.width,
        height: bounding_box.height,
        track_id: track_id.map(str::to_owned),
        track_state,
        zones,
    }
}

fn geometry_view(model: &CameraSpatialModel) -> GeometryView {
    GeometryView {
        observation_region: points(model.observation_region.points()),
        zones: model
            .zones
            .iter()
            .map(|zone| ZoneView {
                zone_id: zone.zone_id.clone(),
                name: zone.name.clone(),
                kind: zone.kind.to_string(),
                points: points(zone.polygon.points()),
            })
            .collect(),
        lines: model
            .lines
            .iter()
            .map(|line| LineView {
                line_id: line.line_id.clone(),
                name: line.name.clone(),
                role: line.role.to_string(),
                start: PointView::from(line.start),
                end: PointView::from(line.end),
            })
            .collect(),
    }
}

fn points(source: &[NormalizedPoint]) -> Vec<PointView> {
    source.iter().copied().map(PointView::from).collect()
}

#[derive(Serialize)]
struct DashboardFrame<'a> {
    #[serde(rename = "type")]
    message_type: &'static str,
    source_id: &'a str,
    frame_id: u64,
    timestamp_ms: u64,
    width: i32,
    height: i32,
    inference_ms: f64,
    processing_fps: f64,
    browser_clients: usize,
    image: String,
    detections: Vec<DetectionView>,
    active_tracks: Vec<TrackView>,
    geometry: Option<GeometryView>,
}

#[derive(Serialize)]
struct DetectionView {
    detection_id: String,
    class_name: String,
    confidence: f32,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    track_id: Option<String>,
    track_state: Option<String>,
    zones: Vec<String>,
}

#[derive(Serialize)]
struct TrackView {
    track_id: String,
    class_name: String,
    state: String,
    observations: usize,
    missed_frames: u32,
}

#[derive(Serialize)]
struct GeometryView {
    observation_region: Vec<PointView>,
    zones: Vec<ZoneView>,
    lines: Vec<LineView>,
}

#[derive(Serialize)]
struct ZoneView {
    zone_id: String,
    name: String,
    kind: String,
    points: Vec<PointView>,
}

#[derive(Serialize)]
struct LineView {
    line_id: String,
    name: String,
    role: String,
    start: PointView,
    end: PointView,
}

#[derive(Clone, Copy, Serialize)]
struct PointView {
    x: f32,
    y: f32,
}

impl From<NormalizedPoint> for PointView {
    fn from(point: NormalizedPoint) -> Self {
        Self {
            x: point.x,
            y: point.y,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpStream};
    use std::time::Duration;

    use super::WebDashboard;

    #[test]
    fn serves_dashboard_health_and_websocket_upgrade() {
        let dashboard = WebDashboard::start("127.0.0.1:0".parse::<SocketAddr>().unwrap()).unwrap();

        let health = request(
            dashboard.local_addr(),
            "GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        );
        assert!(health.starts_with("HTTP/1.1 200 OK"));
        assert!(health.contains("\"status\":\"ok\""));

        let upgrade = request(
            dashboard.local_addr(),
            "GET /ws HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n",
        );
        assert!(upgrade.starts_with("HTTP/1.1 101 Switching Protocols"));
    }

    fn request(address: SocketAddr, request: &str) -> String {
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut response = Vec::new();
        let _ = stream.read_to_end(&mut response);
        String::from_utf8(response).unwrap()
    }
}
