# Arquitectura de Little Brother

## Regla central

Little Brother modela el mundo físico. Las tecnologías externas son fuentes de
observaciones, no entidades centrales del dominio.

```text
Conector -> Normalizador -> Bus de eventos
                              |-> Estado operativo
                              |-> Histórico temporal
                              |-> Relaciones físicas
                              |-> Dashboard
                              |-> Alertas
                              `-> Analítica futura
```

```mermaid
flowchart LR
    subgraph Fuentes[Fuentes externas]
        CAM[Cámara]
        DEV[Dispositivo industrial]
        WMS[WMS]
        MQTT[MQTT]
        SIM[Simulador]
    end

    CAM --> CON[Conectores]
    DEV --> CON
    WMS --> CON
    MQTT --> CON
    SIM --> CON

    CON --> NOR[Normalizador]
    NOR --> ENV[EventEnvelope normalizado]
    ENV --> BUS{{Bus de eventos}}

    BUS --> PER[Router de persistencia]
    BUS --> DASH[Dashboard]
    BUS --> ALT[Motor de alertas]
    BUS --> ANA[Analítica e IA]

    PER --> PG[(PostgreSQL<br/>estado operativo)]
    PER --> REL[(PostgreSQL<br/>relaciones físicas)]
    PER --> TEMP[(ClickHouse / TimescaleDB<br/>histórico temporal)]
```

Los conectores no conocen bases de datos. Un conector transforma su protocolo
particular en un `EventEnvelope` normalizado y lo publica mediante `EventBus`.

## Límites de los crates

### `event-core`

Define el sobre normalizado, la fuente tecnológica neutral y los puertos del
bus. No implementa una tecnología de mensajería durable.

### `transport-core`

Representa plantas, transportadores, conexiones, objetos y movimientos. No
depende de `event-core`, PostgreSQL, ClickHouse ni protocolos externos.

### `persistence-core`

Consume eventos y los clasifica en dominios operativo, temporal o relacional.
Solo define puertos; un adaptador futuro decide tablas, SQL, lotes y motor.

## Evolución de persistencia

```text
Fase 1: adaptadores PostgreSQL para los tres dominios
Fase 2: PostgreSQL operativo + ClickHouse/Timescale histórico
Fase 3: consumidores analíticos adicionales
```

El cambio de fase solo reemplaza o agrega implementaciones de
`PersistenceWriter`. Los conectores, eventos y entidades no cambian.

## Flujo de decisión de persistencia

El evento no selecciona una base de datos. Una política externa determina uno
o varios dominios y el router localiza los escritores registrados para ellos.

```mermaid
flowchart TD
    EVENT[Evento normalizado] --> POLICY[PersistencePolicy]
    POLICY --> OP{¿Modifica estado actual?}
    POLICY --> HIS{¿Debe conservarse como histórico?}
    POLICY --> GRA{¿Modifica una relación física?}

    OP -->|Sí| OPW[Operational writer]
    HIS -->|Sí| HISW[Temporal writer]
    GRA -->|Sí| GRAW[Relational writer]

    OPW --> PG[(PostgreSQL)]
    HISW --> TS[(ClickHouse / TimescaleDB)]
    GRAW --> GRAPH[(PostgreSQL / grafo futuro)]

    OP -->|No| END[Sin escritura en ese dominio]
    HIS -->|No| END
    GRA -->|No| END
```

Un solo evento puede seguir varias ramas. Por ejemplo, la transferencia de una
caja actualiza su ubicación operativa y también se conserva en el histórico.

## Secuencia de una transferencia

```mermaid
sequenceDiagram
    autonumber
    participant C as Conector
    participant N as Normalizador
    participant B as Bus de eventos
    participant P as Router de persistencia
    participant O as Estado operativo
    participant H as Histórico temporal
    participant D as Dashboard

    C->>N: Observación específica del protocolo
    N->>B: EventEnvelope(object.transferred)
    par Actualizar estado
        B->>P: Publicar evento
        P->>O: Actualizar ubicación actual
    and Conservar historia
        P->>H: Insertar evento inmutable
    and Informar
        B->>D: Actualizar vista
    end
```

## Evolución de despliegue

```mermaid
flowchart LR
    subgraph F1[Fase 1]
        B1[Little Brother] --> P1[(PostgreSQL)]
    end

    subgraph F2[Fase 2]
        B2[Little Brother] --> P2[(PostgreSQL)]
        B2 --> C2[(ClickHouse)]
    end

    subgraph F3[Fase 3]
        B3[Little Brother] --> P3[(PostgreSQL)]
        B3 --> C3[(ClickHouse)]
        B3 --> A3[Motor analítico]
    end

    B1 -. evolución .-> B2
    B2 -. evolución .-> B3
```

## Adaptadores futuros

Los adaptadores concretos vivirán fuera de `crates/*-core`:

```text
adapters/
├── bus-nats
├── postgres-operational
├── postgres-relational
└── clickhouse-temporal
```

El bus incluido actualmente es síncrono y en memoria. Sirve para pruebas y para
el prototipo, pero no promete durabilidad. Antes de producción debe sustituirse
por un adaptador durable con confirmación, reintentos, idempotencia y cola de
eventos fallidos.
