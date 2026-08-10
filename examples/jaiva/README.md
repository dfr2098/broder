# Jaiva / DMA_JAIVA

Little Brother ya no usa PostgreSQL. Las detecciones temporales se escriben en
**ClickHouse** y la conexión HTTP **debe pasar por el gateway Jaiva** del
laboratorio `DMA_JAIVA`.

## Variables

| Variable | Rol |
| --- | --- |
| `DMA_JAIVA` | URL base del gateway Jaiva→ClickHouse (obligatoria para persistir) |
| `CLICKHOUSE_USER` / `CLICKHOUSE_PASSWORD` | Credenciales reenviadas al gateway |
| `CLICKHOUSE_DATABASE` | Base lógica (default `temporal`) |

En desarrollo local, `make infra-up` levanta ClickHouse y `DMA_JAIVA` puede
apuntar a `http://127.0.0.1:8123`. En planta, apunta al proxy Jaiva del lab
`DMA_JAIVA`, no al puerto nativo de ClickHouse.

Ver también `broder-vision-to-clickhouse.yaml`.
