# Modelos locales

Los modelos son recursos de ejecución y no programas ejecutables. El motor de
visión debe abrirlos desde esta carpeta mediante OpenCV DNN.

Modelo esperado para el prototipo:

```text
core/yolo/models/yolo11n.onnx
```

El archivo ONNX se mantiene fuera de Git. Debe copiarse al SP durante la
instalación y conservar permisos `0644` para que el proceso Rust pueda leerlo.

Checksum aprobado para el modelo exportado el 20 de julio de 2026:

```text
SHA-256 18fb6fd2901c3dec2ea1a9f38619c66bfe91d51fca4728e13227331be6a95dbf
```

Procedencia del artefacto aprobado:

```text
Modelo fuente:    yolo11n.pt, Ultralytics Assets v8.4.0
Exportador:       Ultralytics 8.4.102
Formato:          ONNX 1.21.0
Opciones:         imgsz=640, opset=12, simplify=true
Imagen Docker:    ultralytics/ultralytics:latest-python-export
Digest de imagen: sha256:810b69e1f297d307f20667b09a66cf652d44ac7c38272b2eea7713b08632f234
Salida esperada:  (1, 84, 8400)
```

El comando de exportación utilizado fue:

```bash
yolo export model=yolo11n.pt format=onnx imgsz=640 opset=12
```

Una exportación realizada con otra versión u otras opciones es un artefacto
distinto y debe validarse antes de reemplazar este checksum.

