#!/usr/bin/env bash

set -u

model_path=${1:-core/yolo/models/yolo11n.onnx}
video_source=${2:-}
spatial_path=${3:-core/vision/config/camera-1.spatial}
expected_ch_port=${4:-8123}
failures=0
minimum_opencv_version=4.13.0

ok() {
    echo "[OK] $1"
}

warn() {
    echo "[WARN] $1"
}

version_at_least() {
    test "$(printf '%s\n' "$2" "$1" | sort -V | head -n 1)" = "$2"
}

fail() {
    echo "[ERROR] $1"
    failures=$((failures + 1))
}

for command_name in cargo rustc pkg-config sha256sum docker curl; do
    if command -v "$command_name" >/dev/null 2>&1; then
        ok "comando disponible: $command_name"
    else
        fail "falta el comando: $command_name"
    fi
done

if pkg-config --exists opencv4 2>/dev/null; then
    opencv_version=$(pkg-config --modversion opencv4)
    if version_at_least "$opencv_version" "$minimum_opencv_version"; then
        ok "OpenCV local: $opencv_version"
    else
        fail "OpenCV $opencv_version es anterior al mínimo $minimum_opencv_version requerido por YOLO11 ONNX"
    fi
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

if test -n "${DMA_JAIVA:-}"; then
    ok "DMA_JAIVA configurada: $DMA_JAIVA"
else
    warn "DMA_JAIVA no está definida; la persistencia quedará desactivada"
fi

published_port=$(docker compose port clickhouse 8123 2>/dev/null || true)
if test -z "$published_port"; then
    if test -n "${DMA_JAIVA:-}"; then
        warn "ClickHouse local no está iniciado; se asume gateway Jaiva externo en DMA_JAIVA"
    else
        fail "ClickHouse no está iniciado; use make infra-up o configure DMA_JAIVA"
    fi
else
    actual_port=${published_port##*:}
    if test "$actual_port" = "$expected_ch_port"; then
        ok "puerto ClickHouse HTTP coherente: $actual_port"
    else
        fail "ClickHouse publica $actual_port pero CLICKHOUSE_HTTP_PORT=$expected_ch_port"
    fi
    if curl -fsS "http://127.0.0.1:${actual_port}/ping" >/dev/null 2>&1; then
        ok "ClickHouse acepta HTTP /ping"
    else
        fail "ClickHouse no responde en /ping"
    fi
fi

if test "$failures" -eq 0; then
    echo "Diagnóstico completado sin errores."
else
    echo "Diagnóstico completado con $failures error(es)."
    exit 1
fi
