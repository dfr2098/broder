# Fases y estado de implementación

## Resumen

| Fase | Objetivo | Estado |
|---:|---|---|
| 1 | Modelo físico de transportadores | Implementada como núcleo y repositorio en memoria |
| 2 | Inferencia YOLO | Implementada con OpenCV DNN, ONNX, 5 FPS y NMS |
| 3 | Seguimiento e identidad | Implementada |
| 4 | Modelo espacial | Implementada con configuración DEMO |
| 5 | Bus, router y PostgreSQL temporal | Implementada para `VisionDetection` |

## Fase 1: modelo físico

Implementado:

- plantas y transportadores;
- dimensiones y estados de transportador;
- grafo dirigido de conexiones;
- objetos de transporte;
- ubicación única por objeto;
- historial reconstruible de movimientos;
- bifurcaciones, convergencias y búsqueda de rutas;
- repositorio en memoria y simulador.

Límite actual: el modelo de transportadores todavía no se persiste en
PostgreSQL y no recibe información de fuentes industriales.

## Fase 2: inferencia YOLO

Implementado:

- archivo de video y URL RTSP;
- muestreo independiente a 5 FPS predeterminados;
- YOLO 11 en formato ONNX;
- OpenCV DNN sobre CPU;
- preprocesamiento y decodificación;
- umbral de confianza;
- NMS por clase;
- cajas normalizadas;
- entidad `VisionDetection`;
- visualización y logs.

Límite actual: `yolo11n.onnx` usa COCO y no posee una clase industrial
específica para pallets. No se ha medido precisión con datos reales de planta.

## Fase 3: seguimiento e identidad

Implementado:

- IDs de track por cámara y ejecución;
- asociación uno a uno por clase, IoU y distancia de centro;
- predicción visual de movimiento;
- estados tentative, confirmed, lost y finished;
- tolerancia a pérdidas breves;
- historial de observaciones y confianza acumulada;
- finalización por frames perdidos o tiempo.

Límite actual: no existe reidentificación entre cámaras ni continuidad después
de reiniciar el proceso.

## Fase 4: modelo espacial

Implementado:

- región útil de observación;
- polígonos y zonas jerárquicas;
- transportador, carril, entrada, salida y zona personalizada;
- líneas virtuales de entrada, salida y límite;
- detección de cruces;
- ancla inferior central de cada track;
- entidad `SpatialTrack`;
- visualización y registros `SPATIAL`;
- configuración externa por cámara.

Límite actual: las coordenadas siguen normalizadas en la imagen. No existe
homografía, calibración métrica ni conversión a metros. La geometría incluida es
demostrativa y el video de prueba no tiene una cámara fija.

## Fase 5: bus y PostgreSQL

Implementado:

- `EventEnvelope<VisionDetection>`;
- bus síncrono dentro de un worker de persistencia;
- entrega asíncrona mediante cola acotada;
- política de dominio temporal;
- `PersistenceRouter` independiente del motor de base;
- adaptador PostgreSQL separado;
- esquema `temporal` y tabla `vision_detection`;
- migración automática idempotente;
- inserción preparada con columnas tipadas y transacciones por lotes;
- flush por tamaño, intervalo y cierre;
- reconexión y reintento de transacción;
- modos `required` y `best-effort`;
- métricas de cola, confirmaciones y descartes;
- índices por tiempo, cámara y clase;
- identificadores de evento por sesión;
- consultas desde `make vision-query`;
- posibilidad de desactivar persistencia con `--no-persistence`.

Límite actual: sólo se persisten detecciones. La cola vive en memoria y no
sobrevive a una caída completa del proceso. No existe política de retención.

## Fuera del alcance actual

No están implementados:

- cálculo de velocidad física a 8 m/s o cualquier otra velocidad;
- detección de cambios de velocidad;
- desalineación de cajas o pallets;
- medición métrica de altura, incluida la validación de un metro máximo;
- reglas industriales y motor de correlación;
- alarmas por Telegram o correo;
- PLC, telegramas, OPC UA, MQTT o WMS;
- correlación entre visión y señales industriales;
- bus durable como NATS, Kafka o Redis Streams;
- persistencia de tracks, resultados espaciales o modelo operativo;
- ClickHouse o TimescaleDB;
- dashboard web, usuarios y permisos;
- reidentificación entre cámaras;
- almacenamiento o compresión de video;
- reproducción y reprocesamiento desde una interfaz de usuario;
- IA predictiva de fallas.

## Siguiente evolución recomendada

Antes de implementar alarmas conviene cerrar, en este orden:

1. calibración de una cámara fija y correspondencia píxel–metro;
2. modelo YOLO entrenado y validado con pallets/cajas reales;
3. persistencia de `VisionTrack` y `SpatialTrack` como eventos separados;
4. bus asíncrono con cola limitada y escritura PostgreSQL por lotes;
5. cálculo de velocidad y dirección con tolerancias configurables;
6. reglas de desalineación y permanencia;
7. correlación con telegramas;
8. generación de alarmas y dashboard.

Esta secuencia evita construir alertas sobre detecciones o mediciones todavía
no calibradas.
