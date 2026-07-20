# Núcleos de procesamiento en Rust

Este directorio es un workspace. Cada proceso o dominio se implementa como un
crate separado para evitar que visión, correlación y topología queden acoplados.

## Componentes actuales

- `crates/event-core`: contrato neutral del sobre y del bus de eventos.
- `crates/persistence-core`: clasifica eventos y los dirige mediante puertos
  abstractos; no contiene SQL ni clientes de bases de datos.
- `crates/persistence-postgres`: implementa el escritor temporal de
  `VisionDetection`, su migración y las consultas preparadas para PostgreSQL.
- `crates/transport-core`: modelo físico de transportadores y movimiento de
  objetos. No conoce cámaras, PLC, WMS ni reglas operativas.
- `crates/vision-core`: entidad `VisionDetection`, cajas normalizadas, muestreo
  temporal y NMS por clase; no depende de OpenCV ni de un modelo específico.
- `crates/tracking-core`: asocia detecciones y mantiene entidades `VisionTrack`
  con estados, confianza e historial mientras permanecen activas. Solo depende
  de `vision-core`.
- `crates/spatial-core`: transforma `VisionTrack` en `SpatialTrack` mediante
  polígonos, zonas jerárquicas y líneas virtuales normalizadas.
- `apps/transport-simulator`: ejecutable pequeño que demuestra el uso del
  núcleo sin infraestructura externa.
- `apps/video-viewer`: visor provisional nativo para inspeccionar videos a una
  frecuencia configurable.
- `apps/vision-inference`: adaptador nativo de archivo/RTSP y YOLO 11 ONNX
  mediante OpenCV DNN sobre CPU.

## Componentes futuros

Los futuros núcleos pueden agregarse como crates hermanos, por ejemplo:

```text
crates/
├── event-core
├── persistence-core
├── persistence-postgres
├── transport-core
├── vision-core
├── tracking-core
├── spatial-core
└── correlation-core  # futuro
```

La comunicación entre núcleos se realizará mediante eventos. Ningún núcleo
debe acceder directamente a las estructuras internas de otro.

## Organización interna del bloque de visión

Cada archivo tiene una responsabilidad concreta y los `main.rs`/`lib.rs` solo
declaran módulos, coordinan dependencias y exponen la API pública:

```text
crates/vision-core/
├── src/lib.rs          fachada pública
├── src/detection.rs    VisionDetection y cajas normalizadas
├── src/nms.rs          supresión de duplicados
├── src/sampler.rs      selección temporal de frames
└── tests/vision.rs     pruebas del contrato

crates/tracking-core/
├── src/lib.rs          fachada pública
├── src/model.rs        VisionTrack, estados y observaciones
├── src/config.rs       umbrales del tracker
├── src/association.rs  predicción y costo espacial
├── src/tracker.rs      ciclo de vida y asignación
├── src/error.rs        errores del núcleo
└── tests/tracker.rs    pruebas de identidad temporal

crates/spatial-core/
├── src/lib.rs          fachada pública
├── src/geometry.rs     puntos y polígonos normalizados
├── src/model.rs        zonas, líneas y SpatialTrack
├── src/spatializer.rs  ubicación y cruces
├── src/error.rs        errores del núcleo
└── tests/spatial.rs    pruebas geométricas

crates/persistence-postgres/
├── src/lib.rs                  fachada del adaptador
├── src/vision_detection.rs     escritor PostgreSQL tipado
└── migrations/0001_...sql     esquema temporal e índices

apps/vision-inference/src/
├── main.rs             composición del proceso
├── config.rs           argumentos de línea de comandos
├── classes.rs          catálogo de clases
├── yolo.rs             OpenCV DNN, preproceso y decodificación
├── stream.rs           captura, muestreo y tracking
├── display.rs          cajas, etiquetas y track_id
├── spatial_config.rs   carga de geometría de cámara
├── persistence.rs      bus, política y composición del router
└── logger.rs           consola y archivo

apps/video-viewer/src/
├── main.rs             composición del visor
├── config.rs           argumentos
├── viewer.rs           reproducción y controles
└── logger.rs           consola y archivo
```

La separación completa y la estrategia de persistencia están descritas en
[`ARCHITECTURE.md`](ARCHITECTURE.md).

## Verificación

```bash
cd core/rs
cargo test --workspace
cargo run -p transport-simulator
```

Desde la raíz del repositorio, una prueba corta del motor de visión se ejecuta
con `make vision-smoke`; `make vision` abre la visualización y
`make vision-headless` procesa sin interfaz.

## Ejecución

Los binarios Rust se ejecutan directamente en el SP o equipo de planta. Este
workspace no requiere ni debe generar una imagen de contenedor:

```bash
cd core/rs
cargo build --release --workspace
./target/release/transport-simulator
```

PostgreSQL y los futuros motores de persistencia se ejecutan aparte mediante
Docker Compose. Los adaptadores reciben sus direcciones mediante configuración;
el dominio no conoce si la base de datos está en un contenedor.
