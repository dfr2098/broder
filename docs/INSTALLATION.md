# Instalación

## Modelo de despliegue

Little Brother usa dos formas de ejecución:

```text
SP / equipo de planta
├── binarios Rust ejecutados directamente
├── OpenCV instalado localmente
├── modelo YOLO ONNX almacenado localmente
└── Docker o Podman
    ├── PostgreSQL 16.4
    └── visualizador Nginx 1.30.4
```

El motor de visión no se ejecuta dentro de un contenedor. Esto permite acceso
directo a cámaras, aceleradores y recursos gráficos del SP. El visualizador sí
se ejecuta en un contenedor y accede al WebSocket del motor mediante un proxy.

## Requisitos

El prototipo ha sido validado con:

| Componente | Versión validada |
|---|---:|
| Rust | 1.97.1 |
| Cargo | 1.97.1 |
| OpenCV | 4.13.0 |
| PostgreSQL | 16.4 |
| Edición Rust | 2024 |

También se requieren:

- compilador C/C++ y `make`;
- `clang`, `libclang` y `pkg-config`, utilizados por el crate de OpenCV;
- Docker Compose o una implementación compatible, como Podman;
- entorno gráfico únicamente para `make viewer` y `make vision`;
- acceso al archivo o a la cámara RTSP configurada.

## Paquetes locales en Debian o Ubuntu

Una instalación habitual de las dependencias nativas es:

```bash
sudo apt update
sudo apt install build-essential make pkg-config clang libclang-dev libopencv-dev ffmpeg
```

> **Importante:** Ubuntu 22.04 instala OpenCV 4.5.4 desde sus repositorios. Esa
> versión permite compilar el proyecto, pero su importador ONNX no puede cargar
> el modelo YOLO11 utilizado por Little Brother. El motor requiere OpenCV 4.13.0
> o posterior. Compruebe siempre la versión después de instalar los paquetes:

```bash
pkg-config --modversion opencv4
```

Si la versión es inferior a 4.13.0, instale una versión actual desde el código
fuente oficial:

Para la demostración se recomienda la instalación local, que no requiere
`sudo` ni reemplaza OpenCV 4.5.4 del sistema:

```bash
make opencv-local
make demo-web
```

El proyecto instala OpenCV en `~/.local/opencv-4.13.0` y configura
automáticamente `PKG_CONFIG_PATH` y `LD_LIBRARY_PATH` al ejecutar comandos del
`Makefile`.

Como alternativa, para una instalación global:

```bash
sudo apt update
sudo apt install build-essential cmake ninja-build pkg-config clang libclang-dev \
  libgtk-3-dev libavcodec-dev libavformat-dev libswscale-dev libv4l-dev \
  libjpeg-dev libpng-dev libtiff-dev libopenblas-dev liblapack-dev

cd /tmp
curl -L https://github.com/opencv/opencv/archive/refs/tags/4.13.0.tar.gz \
  -o opencv-4.13.0.tar.gz
tar -xf opencv-4.13.0.tar.gz

cmake -S /tmp/opencv-4.13.0 -B /tmp/opencv-4.13.0-build -G Ninja \
  -D CMAKE_BUILD_TYPE=Release \
  -D CMAKE_INSTALL_PREFIX=/usr/local \
  -D OPENCV_GENERATE_PKGCONFIG=ON \
  -D BUILD_LIST=core,dnn,imgproc,imgcodecs,videoio,highgui \
  -D BUILD_TESTS=OFF \
  -D BUILD_PERF_TESTS=OFF \
  -D BUILD_EXAMPLES=OFF \
  -D BUILD_opencv_python3=OFF

cmake --build /tmp/opencv-4.13.0-build --parallel
sudo cmake --install /tmp/opencv-4.13.0-build
sudo ldconfig
pkg-config --modversion opencv4
```

Después de reemplazar OpenCV, limpie los artefactos Rust para que el crate
`opencv` vuelva a detectar las bibliotecas y cabeceras instaladas:

```bash
cargo clean --manifest-path core/rs/Cargo.toml
```

Los nombres pueden variar entre distribuciones. Para instalaciones compiladas
de OpenCV se debe conservar el módulo DNN y los módulos `videoio`, `imgproc` y
`highgui`. La guía oficial está en la
[documentación de OpenCV](https://docs.opencv.org/master/d7/d9f/tutorial_linux_install.html).

Rust se administra preferentemente con `rustup`; consulte la
[instalación oficial de Rust](https://rust-lang.org/install.html). Después de
instalarlo, compruebe:

```bash
rustc --version
cargo --version
pkg-config --modversion opencv4
```

Para Docker Engine y Compose siga la
[guía oficial de Docker](https://docs.docker.com/engine/install/). El proyecto
también funciona con un comando `docker` compatible proporcionado por Podman.

## Preparar el repositorio

Desde la raíz del proyecto:

```bash
cp .env.example .env
```

La configuración predeterminada es:

```dotenv
POSTGRES_DB=little_brother
POSTGRES_USER=little_brother
POSTGRES_PASSWORD=change-me
DB_PORT=5432
DATABASE_URL=postgresql://little_brother:change-me@127.0.0.1:5432/little_brother
```

Antes de usar el sistema fuera de un equipo de desarrollo se debe cambiar la
contraseña. `make` reconstruye `DATABASE_URL` a partir de usuario, contraseña,
base y puerto; una `DATABASE_URL` pasada explícitamente en la línea de comandos
tiene prioridad. El archivo `.env` no debe versionarse.

## Instalar el modelo YOLO

El motor espera este archivo:

```text
core/yolo/models/yolo11n.onnx
```

El modelo se copia al SP y no se guarda en Git. Debe ser legible por el usuario
que ejecuta Little Brother. Para verificarlo:

```bash
make verify-model
```

El checksum aprobado está documentado en
[`core/yolo/models/README.md`](../core/yolo/models/README.md). El modelo actual
usa clases COCO: permite comprobar el flujo técnico, pero no sustituye un
modelo entrenado para pallets o cajas de la planta.

## Iniciar PostgreSQL

```bash
make infra-up
docker compose ps
```

PostgreSQL sólo publica el puerto en `127.0.0.1`; no queda expuesto directamente
a la red de planta.

Si `5432` está ocupado:

```bash
make infra-up DB_PORT=55432
```

Use el mismo valor al ejecutar visión:

```bash
make vision-smoke DB_PORT=55432
```

## Compilar y validar

```bash
make check
make release
make doctor
```

`make check` ejecuta pruebas, formato, Clippy y checksum del modelo. Los
binarios optimizados quedan en:

```text
core/rs/target/release/
```

Los ejecutables principales son `vision-inference`, `video-viewer` y
`transport-simulator`.
