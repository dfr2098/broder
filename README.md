# broder

## Arquitectura propuesta

Este proyecto ya está preparado para separar los servicios en dos capas:

- Infraestructura: base de datos PostgreSQL y Redis
- Aplicación: backend y frontend

### Ejecutar con contenedores

```bash
cp .env.example .env
make up
```

### Ejecutar solo infraestructura

```bash
make infra
```

### Ejecutar módulos en el host (por ejemplo worker local)

```bash
make run-worker
```

### Detener todo

```bash
make down
```

La separación está pensada para que puedas usar la misma base con un despliegue tipo cluster, donde la infraestructura se mantiene aparte y los servicios de aplicación se escalan o despliegan por separado.
