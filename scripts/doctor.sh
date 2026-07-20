#!/usr/bin/env bash

set -u

model_path=${1:-core/yolo/models/yolo11n.onnx}
video_source=${2:-}
spatial_path=${3:-core/vision/config/camera-1.spatial}
expected_db_port=${4:-5432}
failures=0

ok() {
    echo "[OK] $1"
}

warn() {
    echo "[WARN] $1"
}

fail() {
    echo "[ERROR] $1"
    failures=$((failures + 1))
}

for command_name in cargo rustc pkg-config sha256sum docker; do
    if command -v "$command_name" >/dev/null 2>&1; then
        ok "comando disponible: $command_name"
    else
        fail "falta el comando: $command_name"
    fi
done

if pkg-config --exists opencv4 2>/dev/null; then
    opencv_version=$(pkg-config --modversion opencv4)
    ok "OpenCV local: $opencv_version"
else
    fail "pkg-config no encuentra OpenCV 4"
fi

if test -r "$model_path"; then
    ok "modelo ONNX legible: $model_path"
else
    fail "modelo ONNX ausente o sin permisos: $model_path"
fi

if (cd core/yolo/models && sha256sum -c SHA256SUMS >/dev/null 2>&1); then
    ok "checksum del modelo aprobado"
else
    fail "el checksum del modelo no coincide"
fi

if test -r "$spatial_path"; then
    ok "configuración espacial legible: $spatial_path"
else
    fail "configuración espacial ausente: $spatial_path"
fi

case "$video_source" in
    rtsp://*|rtsps://*) warn "la conectividad RTSP se valida al abrir el flujo" ;;
    "") warn "no se indicó una fuente de video" ;;
    *)
        if test -r "$video_source"; then
            ok "video de prueba legible"
        else
            fail "video de prueba ausente o sin permisos"
        fi
        ;;
esac

published_port=$(docker compose port db 5432 2>/dev/null || true)
if test -z "$published_port"; then
    fail "PostgreSQL no está iniciado; use make infra-up"
else
    actual_port=${published_port##*:}
    if test "$actual_port" = "$expected_db_port"; then
        ok "puerto PostgreSQL coherente: $actual_port"
    else
        fail "PostgreSQL publica $actual_port pero DB_PORT=$expected_db_port"
    fi
    if docker compose exec -T db pg_isready -U "${POSTGRES_USER:-little_brother}" -d "${POSTGRES_DB:-little_brother}" >/dev/null 2>&1; then
        ok "PostgreSQL acepta conexiones dentro del contenedor"
    else
        fail "PostgreSQL no está listo"
    fi
fi

if test "$failures" -eq 0; then
    echo "Diagnóstico completado sin errores."
else
    echo "Diagnóstico completado con $failures error(es)."
    exit 1
fi
