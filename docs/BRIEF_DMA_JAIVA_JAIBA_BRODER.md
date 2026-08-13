# Brief: DMA_JAIVA ↔ Jaiba ↔ Broder

> Reglas de alcance para agentes. Ajusta rutas locales si tu máquina no usa
> `~/Escritorio/…`.

## Rol de cada pieza

| Pieza | Qué es | Qué NO es |
| --- | --- | --- |
| **Jaiba** (`~/Escritorio/jaiva`) | Motor OSS de conectividad, interpretación y enrutamiento (DAG YAML). Conecta orígenes/destinos. | No es la app de Broder ni la de DMA. No es “capa de acceso a DB” obligatoria de Broder. |
| **DMA_JAIVA** (`~/Escritorio/DMA_JAIVA`) | Lab/puente DMA ↔ Jaiba: sync KPI (Oracle LogOs → lotes → Postgres DMA → Angular). Incluye clone `jaiva/` + sidecar Docker. | No es Broder. No define el producto Broder. |
| **Broder** (este repo) | Productor de eventos / mensajes que Jaiba enruta. En este trabajo el foco es **solo conectividad** (DB y destinos), no cámaras ni visión. | No debe depender de una DB para funcionar. |

## Arquitectura objetivo (Broder)

```text
BRODER ──► JAIBA ──┬──► LLM (opcional)
                   ├──► ClickHouse (histórico, si se persiste)
                   ├──► PostgreSQL / Oracle / Odoo-WMS (sinks opcionales)
                   └──► alertas / drop (si no es relevante)
```

### Reglas

1. Broder puede vivir **sin DB**. Las DB son destinos de Jaiba, no el esqueleto de Broder.
2. Evitar `Broder → DB → servicio → consulta DB → LLM` (latencia e I/O innecesarios).
3. Jaiba decide en el DAG: persistir / analizar / ignorar / drop.
4. ClickHouse = destino analítico recomendado cuando Broder sí necesite histórico
   (vía `put_database` + feature `clickhouse-driver` en Jaiba).
5. DMA_JAIVA = circuito KPI de laboratorio DMA; **no** confundir con el camino Broder.

## DMA_JAIVA (lab) — flujo real hoy

```text
UI/API DMA → POST /heavy/sync/jobs
  → lab_adapter/jaiba_kpi.py
  → /opt/jaiba/jaiba + flows/kpi-*.yaml (Oracle)
  → lotes JSON → dma_logos_* (Postgres) → dashboard Angular
```

Arranque típico:

```bash
cd ~/Escritorio/DMA_JAIVA
./scripts/dma-jaiva.sh start --wait
./scripts/dma-jaiva.sh doctor
./scripts/dma-jaiva.sh sync full --wait
```

Jaiba OSS (conectividad / ClickHouse):

```bash
cd ~/Escritorio/jaiva
cargo run --features clickhouse-driver -- examples/clickhouse-write.yaml
```

## Límites de alcance (importante)

- **En este repo/trabajo:** conectividad Jaiba + destinos DB (incl. ClickHouse).
- **Fuera de alcance:** cámaras, Broder vision, UI de Broder, contratos de imagen.
- **No mezclar** fixes de `DMA_JAIVA` con el OSS `jaiva` salvo sync explícito del clone.

## Docs útiles

| Dónde | Documento |
| --- | --- |
| Jaiba OSS | `docs/stable-stack-tests.md`, `docs/connection-manager.md`, `docs/processors.md` (`put_database`) |
| DMA lab | `DMA_JAIVA/docs/guia-lab.md` |
| Ejemplo ClickHouse | `jaiva/examples/clickhouse-write.yaml` |
| Broder (este repo) | [`examples/jaiva/README.md`](../examples/jaiva/README.md), crate `jaiba-bridge` |

## Criterio de éxito para el agente

- Si el usuario pide **“Broder”**, interpretar: `events in → Jaiba routes → optional DB/LLM/alert`.
- Si pide **“DMA”**, interpretar: lab KPI `Oracle → Jaiba → Postgres DMA`.
- **No** inventar stack de cámaras ni DB obligatoria dentro de Broder.

## Rutas en este entorno Cloud (si aplica)

| Pieza | Ruta habitual del usuario | Nota en Cloud Agent |
| --- | --- | --- |
| Broder | este workspace (`/workspace`) | Repo actual |
| Jaiba OSS | `~/Escritorio/jaiva` | Puede no estar montado; clonar `dfr2098/jaiva` si hace falta |
| DMA_JAIVA | `~/Escritorio/DMA_JAIVA` | Lab fuera de Broder; no está en este repo |
