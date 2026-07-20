# Núcleos de procesamiento en Rust

Este directorio es un workspace. Cada proceso o dominio se implementa como un
crate separado para evitar que visión, correlación y topología queden acoplados.

## Componentes actuales

- `crates/event-core`: contrato neutral del sobre y del bus de eventos.
- `crates/persistence-core`: clasifica eventos y los dirige mediante puertos
  abstractos; no contiene SQL ni clientes de bases de datos.
- `crates/transport-core`: modelo físico de transportadores y movimiento de
  objetos. No conoce cámaras, PLC, WMS ni reglas operativas.
- `apps/transport-simulator`: ejecutable pequeño que demuestra el uso del
  núcleo sin infraestructura externa.

## Componentes futuros

Los futuros núcleos pueden agregarse como crates hermanos, por ejemplo:

```text
crates/
├── event-core
├── persistence-core
├── transport-core
├── vision-core       # futuro
└── correlation-core  # futuro
```

La comunicación entre núcleos se realizará mediante eventos. Ningún núcleo
debe acceder directamente a las estructuras internas de otro.

La separación completa y la estrategia de persistencia están descritas en
[`ARCHITECTURE.md`](ARCHITECTURE.md).

## Verificación

Los crates usan `edition = "2024"`, por lo que se requiere Rust 1.85 o
superior. La versión se fija automáticamente mediante `rust-toolchain.toml`
(si usas `rustup`, se descargará el toolchain necesario al ejecutar `cargo`).

```bash
cd core/rs
cargo test --workspace
cargo run -p transport-simulator
```
