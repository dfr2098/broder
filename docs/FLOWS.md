# Diagramas de flujo de Little Brother

Este documento describe los flujos que ejecuta el prototipo actual. Los
diagramas representan el comportamiento implementado; los componentes futuros
se mencionan únicamente en la sección de alcance.

## Alcance actual

Little Brother recibe video desde un archivo o una cámara RTSP, selecciona
frames a una frecuencia configurable, detecta objetos con YOLO 11, conserva su
identidad temporal, interpreta su posición dentro de una geometría de cámara y
puede guardar las detecciones en PostgreSQL.

Actualmente se persiste `VisionDetection`. Los tracks, los resultados
espaciales, el video, las alarmas y las reglas industriales todavía no se
guardan en la base de datos.

## 1. Flujo general del sistema

Este diagrama muestra la ruta completa de una observación, desde la fuente de
video hasta sus dos salidas actuales: visualización y persistencia.

```mermaid
flowchart TD
    START([Inicio]) --> SRC[Archivo de video o cámara RTSP]
    SRC --> CAP[Captura mediante OpenCV]
    CAP --> SAMPLE{¿Corresponde procesar el frame?}
    SAMPLE -->|No| CAP
    SAMPLE -->|Sí| YOLO[Inferencia YOLO 11]
    YOLO --> FILTER[Filtro de confianza y NMS por clase]
    FILTER --> DET[VisionDetection]

    DET --> TRACK[Tracking de objetos]
    TRACK --> SPATIAL[Interpretación espacial]
    SPATIAL --> OUTPUT[Visualización y logs]

    DET --> PERSIST{¿Persistencia habilitada?}
    PERSIST -->|Sí| QUEUE[Cola acotada]
    QUEUE --> WORKER[Worker de persistencia]
    WORKER --> PG[(PostgreSQL)]
    PERSIST -->|No| CONTINUE[Continuar sin guardar]

    OUTPUT --> NEXT{¿Finalizó el flujo?}
    PG --> NEXT
    CONTINUE --> NEXT
    NEXT -->|No| CAP
    NEXT -->|Sí| CLOSE[Cerrar tracks y vaciar la cola]
    CLOSE --> END([Fin])
```

La detección alimenta dos ramas independientes. Tracking y análisis espacial
producen información para la ejecución y los logs; la persistencia recibe la
detección normalizada directamente, por lo que no espera a que termine el
análisis espacial.

## 2. Inicialización del motor de visión

Antes de leer el primer frame, la aplicación valida la configuración, carga
los recursos opcionales y prepara el motor de inferencia.

```mermaid
flowchart TD
    START([Ejecutar vision-inference]) --> ARGS[Leer y validar argumentos]
    ARGS -->|Inválidos| HELP[Mostrar error y ayuda]
    HELP --> EXIT2([Finalizar con código 2])

    ARGS -->|Válidos| LOG[Abrir archivo de log]
    LOG --> SCFG{¿Hay configuración espacial?}
    SCFG -->|Sí| SMODEL[Cargar geometría de cámara]
    SMODEL --> CAMERA{¿camera_id coincide con source_id?}
    CAMERA -->|No| ERROR([Finalizar con error])
    CAMERA -->|Sí| DATABASE
    SCFG -->|No| DATABASE{¿DATABASE_URL está configurada?}

    DATABASE -->|Sí| PWORKER[Iniciar worker de persistencia]
    PWORKER --> PSTATE{¿PostgreSQL está disponible?}
    PSTATE -->|Sí| READY[Persistencia conectada]
    PSTATE -->|No, best-effort| RETRY[Iniciar y reintentar conexión]
    PSTATE -->|No, required| ERROR
    DATABASE -->|No| DISABLED[Continuar sin persistencia]

    READY --> LOAD
    RETRY --> LOAD
    DISABLED --> LOAD[Cargar clases y modelo ONNX]
    LOAD --> ENGINE[Crear motor OpenCV DNN sobre CPU]
    ENGINE --> STREAM[Iniciar procesamiento del flujo]
```

La configuración espacial y la conexión a PostgreSQL son opcionales. El modelo
ONNX y la fuente de video sí son necesarios para procesar el flujo.

## 3. Procesamiento de cada frame

El muestreador decide según la marca de tiempo de la fuente. En un archivo usa
la posición temporal del video; en RTSP usa un reloj monotónico local.

```mermaid
flowchart TD
    READ[Leer frame] --> VALID{¿Se obtuvo un frame válido?}
    VALID -->|No| FINISH[Finalizar flujo]
    VALID -->|Sí| TIME[Calcular frame_id y timestamp_ms]
    TIME --> SAMPLE{¿El muestreador lo selecciona?}
    SAMPLE -->|No| READ
    SAMPLE -->|Sí| INFER[Ejecutar inferencia]
    INFER --> NORMALIZE[Crear detecciones con cajas normalizadas]
    NORMALIZE --> LOGDET[Registrar detecciones]

    LOGDET --> PUB{¿Hay publicador?}
    PUB -->|Sí| ENQUEUE[Encolar eventos de detección]
    PUB -->|No| TRACK
    ENQUEUE --> TRACK[Procesar tracking]

    TRACK --> LOGTRACK[Registrar tracks activos y finalizados]
    LOGTRACK --> MODEL{¿Hay modelo espacial?}
    MODEL -->|Sí| LOCATE[Localizar tracks actualizados]
    MODEL -->|No| DISPLAY
    LOCATE --> LOGSPACE[Registrar zonas y cruces]
    LOGSPACE --> DISPLAY{¿Visualización habilitada?}

    DISPLAY -->|Sí| DRAW[Dibujar detecciones, tracks y geometría]
    DRAW --> KEY{¿Usuario solicita cerrar?}
    KEY -->|Sí| FINISH
    KEY -->|No| LIMIT
    DISPLAY -->|No| LIVE{¿La fuente es RTSP?}
    LIVE -->|No| WAIT[Regular velocidad del archivo]
    LIVE -->|Sí| LIMIT
    WAIT --> LIMIT{¿Se alcanzó el límite de inferencias?}
    LIMIT -->|Sí| FINISH
    LIMIT -->|No| READ
```

La visualización es opcional. En modo headless se conservan la inferencia, los
logs, el tracking, el análisis espacial configurado y la persistencia.

## 4. Ciclo de vida de un track

El tracker compara detecciones de la misma clase mediante IoU, distancia entre
centros y una predicción de movimiento normalizada.

```mermaid
stateDiagram-v2
    [*] --> Tentative: detección sin asociación
    Tentative --> Confirmed: alcanza observaciones mínimas
    Tentative --> Lost: no se observa
    Confirmed --> Confirmed: vuelve a observarse
    Confirmed --> Lost: pérdida tolerada
    Lost --> Confirmed: reaparece y se asocia
    Lost --> Finished: excede frames o tiempo
    Tentative --> Finished: excede frames o tiempo
    Confirmed --> Finished: cierre del flujo
    Lost --> Finished: cierre del flujo
    Finished --> [*]
```

Un `track_id` conserva la identidad únicamente dentro de una ejecución y una
cámara. El prototipo no realiza reidentificación entre cámaras ni mantiene el
mismo identificador después de reiniciar el proceso.

## 5. Interpretación espacial

El modelo espacial usa como ancla el punto inferior central de la caja de cada
track actualizado.

```mermaid
flowchart LR
    TRACK[VisionTrack actualizado] --> ANCHOR[Calcular ancla inferior central]
    CONFIG[Configuración de cámara] --> GEOMETRY[Región, zonas y líneas]
    ANCHOR --> INSIDE{¿Está dentro de la región útil?}
    GEOMETRY --> INSIDE
    INSIDE --> ZONES[Resolver zonas jerárquicas ocupadas]
    ZONES --> LINES[Detectar cruces de líneas]
    LINES --> RESULT[SpatialTrack]
    RESULT --> LOG[Log y visualización]
```

Las coordenadas se encuentran en el intervalo `0..1`. La configuración DEMO no
equivale a una calibración métrica y no permite calcular velocidad en metros.

## 6. Persistencia de detecciones

Cada `VisionDetection` se envuelve en un evento normalizado y se entrega a un
worker mediante una cola acotada. El worker agrupa escrituras y ejecuta una
transacción al completar el lote, vencer el intervalo de flush o cerrar el
motor.

```mermaid
flowchart TD
    DET[VisionDetection] --> ENV[Crear EventEnvelope]
    ENV --> MODE{Modo de persistencia}

    MODE -->|required| BLOCK[Esperar espacio si la cola está llena]
    MODE -->|best-effort| TRY{¿Hay espacio en la cola?}
    TRY -->|No| DROP[Contabilizar detección descartada]
    TRY -->|Sí| QUEUE[Agregar a la cola]
    BLOCK --> QUEUE

    QUEUE --> WORKER[Worker recibe eventos]
    WORKER --> FLUSH{¿Lote completo, intervalo vencido o cierre?}
    FLUSH -->|No| WORKER
    FLUSH -->|Sí| BUS[Publicar en InMemoryEventBus]
    BUS --> ROUTER[PersistenceRouter]
    ROUTER --> POLICY[Seleccionar dominio temporal]
    POLICY --> WRITER[PostgresVisionDetectionWriter]
    WRITER --> TX{¿Transacción correcta?}
    TX -->|Sí| COMMIT[Confirmar lote y actualizar métricas]
    TX -->|No| RECONNECT[Reconectar y reintentar una vez]
    RECONNECT --> RETRY{¿Reintento correcto?}
    RETRY -->|Sí| COMMIT
    RETRY -->|No| FAILURE[Registrar fallo de persistencia]
```

En modo `required`, la presión de la base de datos se transmite al productor
para evitar pérdidas. En `best-effort`, la visión continúa aunque la cola esté
llena o PostgreSQL no esté disponible, y las pérdidas se reflejan en métricas.

## 7. Cierre controlado

El cierre puede producirse por fin del archivo, error de lectura, solicitud del
usuario o límite de inferencias.

```mermaid
flowchart TD
    STOP[Condición de cierre] --> WINDOW{¿Existe ventana?}
    WINDOW -->|Sí| DESTROY[Cerrar ventana de OpenCV]
    WINDOW -->|No| TRACKS
    DESTROY --> TRACKS[Finalizar todos los tracks activos]
    TRACKS --> LOGTRACKS[Registrar TRACK_FINISHED]
    LOGTRACKS --> PUBLISHER{¿Existe publicador?}
    PUBLISHER -->|Sí| DRAIN[Vaciar cola y cerrar worker]
    PUBLISHER -->|No| METRICS
    DRAIN --> METRICS[Registrar métricas finales]
    METRICS --> SUMMARY[Registrar resumen de ejecución]
    SUMMARY --> END([Fin])
```

Este cierre permite enviar el lote pendiente a PostgreSQL y dejar registrados
los tracks que seguían activos al terminar la fuente.

## Relación entre los resultados

| Resultado | Productor | Uso actual | Persistencia actual |
|---|---|---|---|
| `VisionDetection` | `vision-core` / `vision-inference` | Logs, tracking y eventos | Sí |
| `VisionTrack` | `tracking-core` | Identidad y trayectoria visual | No |
| `SpatialTrack` | `spatial-core` | Zonas, líneas, logs y visualización | No |

## 8. Visualización web en vivo

El backend WebSocket es un consumidor opcional dentro del proceso de visión. El
visualizador se sirve desde un contenedor Nginx que reenvía la conexión hacia
el proceso nativo. Solo se codifican frames JPEG cuando existe al menos un
navegador conectado, evitando ese costo cuando el panel no está en uso.

```mermaid
flowchart LR
    FRAME[Frame procesado] --> DET[VisionDetection]
    DET --> TRACK[TrackingUpdate]
    TRACK --> SPACE[SpatialTrack]
    FRAME --> JPEG[Codificación JPEG]
    DET --> JSON[Mensaje frame]
    TRACK --> JSON
    SPACE --> JSON
    GEOM[Geometría de cámara] --> JSON
    JPEG --> JSON
    JSON --> BUS{{Canal broadcast acotado}}
    BUS --> NATIVE[Backend WebSocket nativo :8081]
    NATIVE --> NGINX[Nginx en Docker :8088]
    NGINX --> WS1[Navegador WebSocket 1]
    NGINX --> WS2[Navegador WebSocket 2]
    NGINX --> WSN[Navegador WebSocket N]
    WS1 --> CANVAS[Canvas: video, cajas y zonas]
```

El canal conserva únicamente una ventana pequeña de mensajes. Si un navegador
es demasiado lento, omite frames atrasados y continúa con información reciente;
no bloquea la inferencia ni la persistencia.

## 9. Validación con el video de prueba

El repositorio está configurado localmente para usar
`video prueba/WhatsApp Video 2026-07-20 at 10.07.07 AM.mp4`. El archivo dura
aproximadamente 14 segundos y no se versiona en Git.

```mermaid
flowchart TD
    FILE[Video MP4 de prueba] --> EXISTS{¿El archivo es legible?}
    EXISTS -->|No| COPY[Copiar el video o definir VIDEO]
    COPY --> EXISTS
    EXISTS -->|Sí| INFO[Ejecutar make viewer-info]
    INFO --> OPEN{¿OpenCV abre el video?}
    OPEN -->|No| DEPS[Revisar OpenCV, pkg-config y códecs]
    DEPS --> INFO
    OPEN -->|Sí| DOCTOR[Ejecutar make doctor]
    DOCTOR --> READY{¿Modelo, geometría y servicios están listos?}
    READY -->|No| FIX[Instalar o configurar recursos faltantes]
    FIX --> DOCTOR
    READY -->|Sí| SMOKE[Ejecutar make vision-smoke]
    SMOKE --> LOGS[Revisar detecciones, tracks y métricas]
    LOGS --> QUERY{¿PostgreSQL está habilitado?}
    QUERY -->|Sí| DB[Ejecutar make vision-query]
    QUERY -->|No| END([Prueba local terminada])
    DB --> END
```

`viewer-info` valida la lectura del archivo sin abrir una ventana. Después,
`vision-smoke` limita la ejecución a seis inferencias para comprobar rápidamente
la ruta de detección, tracking, análisis espacial y persistencia configurada.

Para los límites entre crates, contratos y decisiones arquitectónicas, consulte
[Arquitectura de Little Brother](../core/rs/ARCHITECTURE.md). Para comandos de
ejecución y diagnóstico, consulte [Operación](OPERATIONS.md).
