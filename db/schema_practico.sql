-- =====================================================
-- SCHEMA PRÁCTICO PARA SISTEMA DE VISIÓN + CORRELACIÓN
-- PostgreSQL
-- =====================================================
-- Este script crea el modelo, inserta datos de ejemplo
-- y deja listo para probar con un dashboard o API.

CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- =====================================================
-- LIMPIEZA (opcional, para reiniciar)
-- =====================================================
DROP VIEW IF EXISTS vw_eventos_dashboard;
DROP TABLE IF EXISTS alerta CASCADE;
DROP TABLE IF EXISTS evento CASCADE;
DROP TABLE IF EXISTS mensaje_externo CASCADE;
DROP TABLE IF EXISTS seguimiento_objeto CASCADE;
DROP TABLE IF EXISTS deteccion_objeto CASCADE;
DROP TABLE IF EXISTS captura_vision CASCADE;
DROP TABLE IF EXISTS telegrama CASCADE;
DROP TABLE IF EXISTS posicion_objeto CASCADE;
DROP TABLE IF EXISTS dispositivo CASCADE;
DROP TABLE IF EXISTS controlador_transportador CASCADE;
DROP TABLE IF EXISTS controlador CASCADE;
DROP TABLE IF EXISTS conexion_transportador CASCADE;
DROP TABLE IF EXISTS transportador CASCADE;
DROP TABLE IF EXISTS area CASCADE;
DROP TABLE IF EXISTS planta CASCADE;
DROP TABLE IF EXISTS fuente_dato CASCADE;
DROP TABLE IF EXISTS tipo_evento CASCADE;
DROP TABLE IF EXISTS objeto CASCADE;

-- =====================================================
-- 1) PLANTA
-- =====================================================
CREATE TABLE planta (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    codigo VARCHAR(50) NOT NULL UNIQUE,
    nombre VARCHAR(150) NOT NULL,
    direccion VARCHAR(255),
    estado VARCHAR(20) NOT NULL DEFAULT 'activo'
        CHECK (estado IN ('activo', 'inactivo'))
);

-- =====================================================
-- 2) ÁREA
-- =====================================================
CREATE TABLE area (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    planta_id UUID NOT NULL REFERENCES planta(id) ON DELETE RESTRICT,
    codigo VARCHAR(50) NOT NULL UNIQUE,
    nombre VARCHAR(150) NOT NULL,
    descripcion TEXT
);

-- =====================================================
-- 3) TRANSPORTADOR
-- =====================================================
CREATE TABLE transportador (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    area_id UUID NOT NULL REFERENCES area(id) ON DELETE RESTRICT,
    codigo VARCHAR(50) NOT NULL UNIQUE,
    nombre VARCHAR(150) NOT NULL,
    tipo VARCHAR(50),
    longitud NUMERIC(10,2),
    ancho NUMERIC(10,2),
    sentido VARCHAR(50),
    velocidad_nominal NUMERIC(10,2),
    estado VARCHAR(20) NOT NULL DEFAULT 'activo'
        CHECK (estado IN ('activo', 'inactivo'))
);

-- =====================================================
-- 4) CONEXIÓN ENTRE TRANSPORTADORES
-- =====================================================
CREATE TABLE conexion_transportador (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    origen_id UUID NOT NULL REFERENCES transportador(id) ON DELETE CASCADE,
    destino_id UUID NOT NULL REFERENCES transportador(id) ON DELETE CASCADE,
    tipo_conexion VARCHAR(50) NOT NULL,
    distancia NUMERIC(10,2),
    tiempo_estimado INT,
    CONSTRAINT chk_conexion_distinta CHECK (origen_id <> destino_id)
);

-- =====================================================
-- 5) CONTROLADOR
-- =====================================================
CREATE TABLE controlador (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    codigo VARCHAR(50) NOT NULL UNIQUE,
    nombre VARCHAR(150) NOT NULL,
    tipo VARCHAR(50) NOT NULL,
    fabricante VARCHAR(100),
    modelo VARCHAR(100),
    direccion_red VARCHAR(255),
    estado VARCHAR(20) NOT NULL DEFAULT 'activo'
        CHECK (estado IN ('activo', 'inactivo'))
);

-- =====================================================
-- 6) RELACIÓN CONTROLADOR - TRANSPORTADOR
-- =====================================================
CREATE TABLE controlador_transportador (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    controlador_id UUID NOT NULL REFERENCES controlador(id) ON DELETE CASCADE,
    transportador_id UUID NOT NULL REFERENCES transportador(id) ON DELETE CASCADE,
    funcion VARCHAR(50) NOT NULL DEFAULT 'principal',
    UNIQUE (controlador_id, transportador_id)
);

-- =====================================================
-- 7) DISPOSITIVO
-- =====================================================
CREATE TABLE dispositivo (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    transportador_id UUID NOT NULL REFERENCES transportador(id) ON DELETE CASCADE,
    tipo VARCHAR(50) NOT NULL
        CHECK (tipo IN ('camara', 'scanner', 'sensor', 'rfid', 'bascula', 'otro')),
    codigo VARCHAR(50) NOT NULL UNIQUE,
    nombre VARCHAR(150) NOT NULL,
    estado VARCHAR(20) NOT NULL DEFAULT 'activo'
        CHECK (estado IN ('activo', 'inactivo')),
    ubicacion VARCHAR(150)
);

-- =====================================================
-- 8) OBJETO
-- =====================================================
CREATE TABLE objeto (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    codigo VARCHAR(50) NOT NULL UNIQUE,
    tipo VARCHAR(50) NOT NULL
        CHECK (tipo IN ('caja', 'tarima', 'contenedor', 'producto')),
    estado VARCHAR(20) NOT NULL DEFAULT 'activo'
        CHECK (estado IN ('activo', 'inactivo', 'procesado')),
    fecha_creacion TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- =====================================================
-- 9) POSICIÓN DEL OBJETO
-- =====================================================
CREATE TABLE posicion_objeto (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    objeto_id UUID NOT NULL REFERENCES objeto(id) ON DELETE CASCADE,
    transportador_id UUID REFERENCES transportador(id) ON DELETE SET NULL,
    zona VARCHAR(100),
    fecha_hora TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    posicion_x NUMERIC(10,2),
    posicion_y NUMERIC(10,2),
    velocidad NUMERIC(10,2),
    direccion VARCHAR(50)
);

-- =====================================================
-- 10) TELEGRAMA
-- =====================================================
CREATE TABLE telegrama (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    controlador_id UUID REFERENCES controlador(id) ON DELETE SET NULL,
    transportador_id UUID REFERENCES transportador(id) ON DELETE SET NULL,
    fecha_hora TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    tipo VARCHAR(50),
    direccion VARCHAR(20) NOT NULL
        CHECK (direccion IN ('entrada', 'salida')),
    contenido_original TEXT NOT NULL,
    estado_procesamiento VARCHAR(20) NOT NULL DEFAULT 'pendiente'
        CHECK (estado_procesamiento IN ('pendiente', 'procesado', 'error'))
);

-- =====================================================
-- 11) TIPO DE EVENTO
-- =====================================================
CREATE TABLE tipo_evento (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    codigo VARCHAR(50) NOT NULL UNIQUE,
    nombre VARCHAR(150) NOT NULL,
    descripcion TEXT
);

-- =====================================================
-- 12) FUENTE DE DATO
-- =====================================================
CREATE TABLE fuente_dato (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    codigo VARCHAR(50) NOT NULL UNIQUE,
    tipo VARCHAR(30) NOT NULL
        CHECK (tipo IN ('camara_ip', 'vision', 'plc', 'handheld', 'telegram', 'wcs')),
    nombre VARCHAR(150) NOT NULL,
    estado VARCHAR(20) NOT NULL DEFAULT 'activo'
        CHECK (estado IN ('activo', 'inactivo'))
);

-- =====================================================
-- 13) CAPTURA DE VISIÓN
-- =====================================================
CREATE TABLE captura_vision (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fuente_id UUID REFERENCES fuente_dato(id) ON DELETE SET NULL,
    transportador_id UUID REFERENCES transportador(id) ON DELETE SET NULL,
    ts TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    frame_id VARCHAR(100),
    ruta_imagen TEXT,
    confidence NUMERIC(5,4),
    estado VARCHAR(20) NOT NULL DEFAULT 'procesado'
        CHECK (estado IN ('procesado', 'error', 'descartado'))
);

-- =====================================================
-- 14) DETECCIÓN DE OBJETO
-- =====================================================
CREATE TABLE deteccion_objeto (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    captura_id UUID REFERENCES captura_vision(id) ON DELETE CASCADE,
    objeto_id UUID REFERENCES objeto(id) ON DELETE SET NULL,
    transportador_id UUID REFERENCES transportador(id) ON DELETE SET NULL,
    tipo_detec VARCHAR(50),
    x NUMERIC(10,2),
    y NUMERIC(10,2),
    confidence NUMERIC(5,4),
    metadata JSONB
);

-- =====================================================
-- 15) SEGUIMIENTO DE OBJETO
-- =====================================================
CREATE TABLE seguimiento_objeto (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    objeto_id UUID REFERENCES objeto(id) ON DELETE CASCADE,
    transportador_id UUID REFERENCES transportador(id) ON DELETE SET NULL,
    estado VARCHAR(30) NOT NULL DEFAULT 'en_ruta'
        CHECK (estado IN ('en_ruta', 'detenido', 'perdido', 'salida', 'entrada')),
    ultimo_ts TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ultimo_x NUMERIC(10,2),
    ultimo_y NUMERIC(10,2),
    velocidad NUMERIC(10,2),
    direccion VARCHAR(50),
    metadata JSONB
);

-- =====================================================
-- 16) MENSAJE EXTERNO (PLC / handheld / telegram)
-- =====================================================
CREATE TABLE mensaje_externo (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fuente_id UUID REFERENCES fuente_dato(id) ON DELETE SET NULL,
    transportador_id UUID REFERENCES transportador(id) ON DELETE SET NULL,
    ts TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    tipo VARCHAR(50),
    direccion VARCHAR(20) NOT NULL
        CHECK (direccion IN ('entrada', 'salida')),
    contenido JSONB NOT NULL,
    estado_procesamiento VARCHAR(20) NOT NULL DEFAULT 'recibido'
        CHECK (estado_procesamiento IN ('recibido', 'procesado', 'error'))
);

-- =====================================================
-- 17) EVENTO DE NEGOCIO
-- =====================================================
CREATE TABLE evento (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tipo_evento_id UUID REFERENCES tipo_evento(id) ON DELETE RESTRICT,
    origen VARCHAR(30) NOT NULL
        CHECK (origen IN ('vision', 'plc', 'handheld', 'telegram')),
    fuente_id UUID REFERENCES fuente_dato(id) ON DELETE SET NULL,
    transportador_id UUID REFERENCES transportador(id) ON DELETE SET NULL,
    objeto_id UUID REFERENCES objeto(id) ON DELETE SET NULL,
    telegrama_id UUID REFERENCES telegrama(id) ON DELETE SET NULL,
    ts TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    prioridad INT NOT NULL DEFAULT 1 CHECK (prioridad BETWEEN 1 AND 5),
    estado VARCHAR(20) NOT NULL DEFAULT 'nuevo'
        CHECK (estado IN ('nuevo', 'procesado', 'error', 'ignorado')),
    detalle JSONB
);

-- =====================================================
-- 18) ALERTA
-- =====================================================
CREATE TABLE alerta (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    evento_id UUID REFERENCES evento(id) ON DELETE CASCADE,
    canal VARCHAR(30) NOT NULL
        CHECK (canal IN ('telegram', 'correo', 'dashboard')),
    estado VARCHAR(20) NOT NULL DEFAULT 'pendiente'
        CHECK (estado IN ('pendiente', 'enviada', 'fallida')),
    creado_en TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- =====================================================
-- ÍNDICES
-- =====================================================
CREATE INDEX idx_area_planta_id ON area(planta_id);
CREATE INDEX idx_transportador_area_id ON transportador(area_id);
CREATE INDEX idx_conexion_origen ON conexion_transportador(origen_id);
CREATE INDEX idx_conexion_destino ON conexion_transportador(destino_id);
CREATE INDEX idx_controlador_transportador_transportador ON controlador_transportador(transportador_id);
CREATE INDEX idx_dispositivo_transportador ON dispositivo(transportador_id);
CREATE INDEX idx_posicion_objeto_objeto ON posicion_objeto(objeto_id);
CREATE INDEX idx_posicion_objeto_transportador ON posicion_objeto(transportador_id);
CREATE INDEX idx_telegrama_transportador ON telegrama(transportador_id);
CREATE INDEX idx_captura_vision_ts ON captura_vision(ts);
CREATE INDEX idx_deteccion_objeto_captura ON deteccion_objeto(captura_id);
CREATE INDEX idx_seguimiento_objeto_objeto ON seguimiento_objeto(objeto_id);
CREATE INDEX idx_mensaje_externo_ts ON mensaje_externo(ts);
CREATE INDEX idx_evento_ts ON evento(ts);
CREATE INDEX idx_evento_estado ON evento(estado);
CREATE INDEX idx_alerta_estado ON alerta(estado);

-- =====================================================
-- DATOS DE EJEMPLO
-- =====================================================
INSERT INTO planta (id, codigo, nombre, direccion, estado) VALUES
('11111111-1111-1111-1111-111111111111', 'PLN-001', 'Planta Norte', 'Av. Industrial 123', 'activo');

INSERT INTO area (id, planta_id, codigo, nombre, descripcion) VALUES
('22222222-2222-2222-2222-222222222222', '11111111-1111-1111-1111-111111111111', 'AREA-001', 'Recepción', 'Zona de entrada de mercancías'),
('33333333-3333-3333-3333-333333333333', '11111111-1111-1111-1111-111111111111', 'AREA-002', 'Clasificación', 'Zona de inspección y separación'),
('44444444-4444-4444-4444-444444444444', '11111111-1111-1111-1111-111111111111', 'AREA-003', 'Salida', 'Zona de despacho');

INSERT INTO transportador (id, area_id, codigo, nombre, tipo, longitud, ancho, sentido, velocidad_nominal, estado) VALUES
('55555555-5555-5555-5555-555555555555', '22222222-2222-2222-2222-222222222222', 'TRN-001', 'Transportador A', 'corredor', 12.50, 1.20, 'derecha', 0.80, 'activo'),
('66666666-6666-6666-6666-666666666666', '33333333-3333-3333-3333-333333333333', 'TRN-002', 'Transportador B', 'corredor', 8.00, 1.20, 'izquierda', 0.70, 'activo'),
('77777777-7777-7777-7777-777777777777', '44444444-4444-4444-4444-444444444444', 'TRN-003', 'Transportador C', 'corredor', 10.00, 1.20, 'derecha', 0.75, 'activo');

INSERT INTO conexion_transportador (id, origen_id, destino_id, tipo_conexion, distancia, tiempo_estimado) VALUES
('88888888-8888-8888-8888-888888888888', '55555555-5555-5555-5555-555555555555', '66666666-6666-6666-6666-666666666666', 'bifurcación', 4.50, 12),
('99999999-9999-9999-9999-999999999999', '66666666-6666-6666-6666-666666666666', '77777777-7777-7777-7777-777777777777', 'unión', 3.20, 8);

INSERT INTO controlador (id, codigo, nombre, tipo, fabricante, modelo, direccion_red, estado) VALUES
('aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'PLC-001', 'PLC Principal', 'PLC', 'Siemens', 'S7-1200', '192.168.10.10', 'activo'),
('bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb', 'IPC-001', 'IPC Vision', 'IPC', 'Dell', 'OptiPlex', '192.168.10.20', 'activo');

INSERT INTO controlador_transportador (id, controlador_id, transportador_id, funcion) VALUES
('cccccccc-cccc-cccc-cccc-cccccccccccc', 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', '55555555-5555-5555-5555-555555555555', 'principal'),
('dddddddd-dddd-dddd-dddd-dddddddddddd', 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', '66666666-6666-6666-6666-666666666666', 'respaldo'),
('eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee', 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb', '77777777-7777-7777-7777-777777777777', 'monitoreo');

INSERT INTO dispositivo (id, transportador_id, tipo, codigo, nombre, estado, ubicacion) VALUES
('ffffffff-ffff-ffff-ffff-ffffffffffff', '55555555-5555-5555-5555-555555555555', 'camara', 'CAM-001', 'Cámara Entrada', 'activo', 'lado izquierdo'),
('10101010-1010-1010-1010-101010101010', '66666666-6666-6666-6666-666666666666', 'sensor', 'SEN-001', 'Sensor de presencia', 'activo', 'centro'),
('11111111-1111-1111-1111-111111111111', '77777777-7777-7777-7777-777777777777', 'scanner', 'SCR-001', 'Scanner de salida', 'activo', 'extremo');

INSERT INTO objeto (id, codigo, tipo, estado, fecha_creacion) VALUES
('12121212-1212-1212-1212-121212121212', 'OBJ-001', 'caja', 'activo', NOW()),
('13131313-1313-1313-1313-131313131313', 'OBJ-002', 'tarima', 'activo', NOW());

INSERT INTO tipo_evento (id, codigo, nombre, descripcion) VALUES
('14141414-1414-1414-1414-141414141414', 'CAJA_DETECTADA', 'Caja detectada', 'Se detecta un objeto en el transportador'),
('15151515-1515-1515-1515-151515151515', 'CAJA_DETENIDA', 'Caja detenida', 'El objeto se detiene por un evento de control'),
('16161616-1616-1616-1616-161616161616', 'TELEGRAMA_RECIBIDO', 'Telegrama recibido', 'Se recibe un mensaje del controlador');

INSERT INTO fuente_dato (id, codigo, tipo, nombre, estado) VALUES
('17171717-1717-1717-1717-171717171717', 'SRC-CAM-01', 'camara_ip', 'Cámara IP Entrada', 'activo'),
('18181818-1818-1818-1818-181818181818', 'SRC-VIS-01', 'vision', 'Servicio de visión', 'activo'),
('19191919-1919-1919-1919-191919191919', 'SRC-PLC-01', 'plc', 'PLC Principal', 'activo'),
('20202020-2020-2020-2020-202020202020', 'SRC-HH-01', 'handheld', 'Handheld de operación', 'activo');

INSERT INTO telegrama (id, controlador_id, transportador_id, fecha_hora, tipo, direccion, contenido_original, estado_procesamiento) VALUES
('21212121-2121-2121-2121-212121212121', 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', '55555555-5555-5555-5555-555555555555', NOW(), 'status', 'entrada', '{"estado":"ok","codigo":"PLC-001"}', 'procesado');

INSERT INTO captura_vision (id, fuente_id, transportador_id, ts, frame_id, ruta_imagen, confidence, estado) VALUES
('22222222-2222-2222-2222-222222222222', '17171717-1717-1717-1717-171717171717', '55555555-5555-5555-5555-555555555555', NOW(), 'frame-001', '/imagenes/frame-001.jpg', 0.9832, 'procesado'),
('23232323-2323-2323-2323-232323232323', '18181818-1818-1818-1818-181818181818', '66666666-6666-6666-6666-666666666666', NOW(), 'frame-002', '/imagenes/frame-002.jpg', 0.9567, 'procesado');

INSERT INTO deteccion_objeto (id, captura_id, objeto_id, transportador_id, tipo_detec, x, y, confidence, metadata) VALUES
('24242424-2424-2424-2424-242424242424', '22222222-2222-2222-2222-222222222222', '12121212-1212-1212-1212-121212121212', '55555555-5555-5555-5555-555555555555', 'deteccion', 120.50, 45.20, 0.9832, '{"clase":"caja"}'::jsonb),
('25252525-2525-2525-2525-252525252525', '23232323-2323-2323-2323-232323232323', '13131313-1313-1313-1313-131313131313', '66666666-6666-6666-6666-666666666666', 'deteccion', 340.00, 80.10, 0.9567, '{"clase":"tarima"}'::jsonb);

INSERT INTO seguimiento_objeto (id, objeto_id, transportador_id, estado, ultimo_ts, ultimo_x, ultimo_y, velocidad, direccion, metadata) VALUES
('26262626-2626-2626-2626-262626262626', '12121212-1212-1212-1212-121212121212', '55555555-5555-5555-5555-555555555555', 'en_ruta', NOW(), 120.50, 45.20, 0.80, 'derecha', '{"origen":"vision"}'::jsonb),
('27272727-2727-2727-2727-272727272727', '13131313-1313-1313-1313-131313131313', '66666666-6666-6666-6666-666666666666', 'detenido', NOW(), 340.00, 80.10, 0.00, 'n/a', '{"origen":"plc"}'::jsonb);

INSERT INTO mensaje_externo (id, fuente_id, transportador_id, ts, tipo, direccion, contenido, estado_procesamiento) VALUES
('28282828-2828-2828-2828-282828282828', '19191919-1919-1919-1919-191919191919', '55555555-5555-5555-5555-555555555555', NOW(), 'status', 'entrada', '{"codigo":"PLC-001","estado":"run"}'::jsonb, 'procesado'),
('29292929-2929-2929-2929-292929292929', '20202020-2020-2020-2020-202020202020', '66666666-6666-6666-6666-666666666666', NOW(), 'scan', 'entrada', '{"usuario":"ops01","codigo":"OBJ-002"}'::jsonb, 'procesado');

INSERT INTO evento (id, tipo_evento_id, origen, fuente_id, transportador_id, objeto_id, telegrama_id, ts, prioridad, estado, detalle) VALUES
('30303030-3030-3030-3030-303030303030', '14141414-1414-1414-1414-141414141414', 'vision', '18181818-1818-1818-1818-181818181818', '55555555-5555-5555-5555-555555555555', '12121212-1212-1212-1212-121212121212', '21212121-2121-2121-2121-212121212121', NOW(), 2, 'procesado', '{"mensaje":"Caja detectada en el transportador A"}'::jsonb),
('31313131-3131-3131-3131-313131313131', '15151515-1515-1515-1515-151515151515', 'plc', '19191919-1919-1919-1919-191919191919', '66666666-6666-6666-6666-666666666666', '13131313-1313-1313-1313-131313131313', '21212121-2121-2121-2121-212121212121', NOW(), 3, 'nuevo', '{"mensaje":"Objeto detenido por condición de control"}'::jsonb),
('32323232-3232-3232-3232-323232323232', '16161616-1616-1616-1616-161616161616', 'telegram', '19191919-1919-1919-1919-191919191919', '55555555-5555-5555-5555-555555555555', '12121212-1212-1212-1212-121212121212', '21212121-2121-2121-2121-212121212121', NOW(), 1, 'procesado', '{"mensaje":"Telegrama recibido desde PLC"}'::jsonb);

INSERT INTO alerta (id, evento_id, canal, estado, creado_en) VALUES
('33333333-3333-3333-3333-333333333333', '31313131-3131-3131-3131-313131313131', 'telegram', 'pendiente', NOW()),
('34343434-3434-3434-3434-343434343434', '30303030-3030-3030-3030-303030303030', 'dashboard', 'enviada', NOW());

CREATE VIEW vw_eventos_dashboard AS
SELECT
    e.id,
    te.nombre AS tipo_evento,
    e.origen,
    t.codigo AS transportador,
    o.codigo AS objeto,
    e.ts,
    e.prioridad,
    e.estado
FROM evento e
LEFT JOIN tipo_evento te ON te.id = e.tipo_evento_id
LEFT JOIN transportador t ON t.id = e.transportador_id
LEFT JOIN objeto o ON o.id = e.objeto_id;
