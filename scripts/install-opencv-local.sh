#!/usr/bin/env bash

set -euo pipefail

opencv_version=${OPENCV_VERSION:-4.13.0}
install_prefix=${OPENCV_INSTALL_PREFIX:-"$HOME/.local/opencv-$opencv_version"}
build_root=${OPENCV_BUILD_ROOT:-"/tmp/little-brother-opencv-$opencv_version"}
source_dir="$build_root/opencv-$opencv_version"
build_dir="$build_root/build"
archive="$build_root/opencv-$opencv_version.tar.gz"
build_jobs=${OPENCV_BUILD_JOBS:-8}

for required_command in cmake make curl tar pkg-config; do
    if ! command -v "$required_command" >/dev/null 2>&1; then
        echo "[ERROR] falta el comando: $required_command" >&2
        exit 1
    fi
done

if test -f "$install_prefix/lib/pkgconfig/opencv4.pc"; then
    installed_version=$(PKG_CONFIG_PATH="$install_prefix/lib/pkgconfig" pkg-config --modversion opencv4)
    if test "$installed_version" = "$opencv_version"; then
        echo "[OK] OpenCV $opencv_version ya está instalado en $install_prefix"
        exit 0
    fi
fi

mkdir -p "$build_root"

if ! test -f "$archive"; then
    echo "[INFO] descargando OpenCV $opencv_version"
    curl --fail --location --retry 3 \
        "https://github.com/opencv/opencv/archive/refs/tags/$opencv_version.tar.gz" \
        --output "$archive"
fi

if ! test -f "$source_dir/CMakeLists.txt"; then
    echo "[INFO] extrayendo fuentes"
    tar -xf "$archive" -C "$build_root"
fi

echo "[INFO] configurando instalación local en $install_prefix"
cmake -S "$source_dir" -B "$build_dir" -G "Unix Makefiles" \
    -D CMAKE_BUILD_TYPE=Release \
    -D CMAKE_INSTALL_PREFIX="$install_prefix" \
    -D OPENCV_GENERATE_PKGCONFIG=ON \
    -D OPENCV_LIB_INSTALL_PATH=lib \
    -D BUILD_LIST=core,dnn,imgproc,imgcodecs,videoio,highgui \
    -D BUILD_TESTS=OFF \
    -D BUILD_PERF_TESTS=OFF \
    -D BUILD_EXAMPLES=OFF \
    -D BUILD_opencv_apps=OFF \
    -D BUILD_opencv_python3=OFF \
    -D BUILD_JAVA=OFF \
    -D BUILD_PROTOBUF=ON \
    -D WITH_GTK=OFF \
    -D WITH_QT=OFF \
    -D WITH_OPENGL=OFF \
    -D WITH_OPENCL=OFF \
    -D WITH_IPP=OFF \
    -D WITH_FFMPEG=ON \
    -D WITH_GSTREAMER=OFF

echo "[INFO] compilando con $build_jobs trabajos paralelos"
cmake --build "$build_dir" --parallel "$build_jobs"
cmake --install "$build_dir"

installed_version=$(PKG_CONFIG_PATH="$install_prefix/lib/pkgconfig" pkg-config --modversion opencv4)
echo "[OK] OpenCV $installed_version instalado en $install_prefix"
