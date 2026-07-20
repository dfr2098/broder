# Modelos locales

Los modelos son recursos de ejecución y no programas ejecutables. El motor de
visión debe abrirlos desde esta carpeta mediante OpenCV DNN.

Modelo esperado para el prototipo:

```text
core/yolo/models/yolo11n.onnx
```

El archivo ONNX se mantiene fuera de Git. Debe copiarse al SP durante la
instalación y conservar permisos `0644` para que el proceso Rust pueda leerlo.

Checksum aprobado para el modelo exportado el 19 de julio de 2026:

```text
SHA-256 3769441905508d1be4e5c0b82809dd88cabbdb89899f25fd96edc379055bdc44
```

