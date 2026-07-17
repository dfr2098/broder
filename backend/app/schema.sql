-- =====================================================================
-- Esquema del sistema de transportadores (PostgreSQL)
--
-- Este archivo es la fuente de verdad de la estructura de la base de
-- datos. Se ejecuta de dos formas:
--   1) Automáticamente por el contenedor de Postgres al inicializarse
--      (montado en /docker-entrypoint-initdb.d/schema.sql).
--   2) Manualmente desde Python:  python -m backend.app.create_tables
--
-- Es idempotente: se puede ejecutar varias veces sin error.
--
-- Los identificadores usan UUIDv7: son consecutivos (ordenados por
-- tiempo de creación, por lo que el índice sigue siendo eficiente) pero
-- NO son 1,2,3, lo que dificulta la enumeración por un atacante (IDOR).
-- =====================================================================

-- ---------------------------------------------------------------------
-- Función uuidv7(): genera UUID versión 7 (RFC 9562).
-- Postgres 16 no la trae de forma nativa (llega en Postgres 18).
-- Usa gen_random_uuid() (core desde PG13) como fuente de aleatoriedad,
-- así no requiere la extensión pgcrypto.
-- ---------------------------------------------------------------------
CREATE OR REPLACE FUNCTION uuidv7() RETURNS uuid AS $$
DECLARE
    ts_ms  bigint;
    ubytes bytea;
BEGIN
    ts_ms := (extract(epoch from clock_timestamp()) * 1000)::bigint;
    -- 16 bytes aleatorios; sobreescribimos los primeros 48 bits con el timestamp
    ubytes := decode(replace(gen_random_uuid()::text, '-', ''), 'hex');
    ubytes := set_byte(ubytes, 0, ((ts_ms >> 40) & 255)::int);
    ubytes := set_byte(ubytes, 1, ((ts_ms >> 32) & 255)::int);
    ubytes := set_byte(ubytes, 2, ((ts_ms >> 24) & 255)::int);
    ubytes := set_byte(ubytes, 3, ((ts_ms >> 16) & 255)::int);
    ubytes := set_byte(ubytes, 4, ((ts_ms >> 8)  & 255)::int);
    ubytes := set_byte(ubytes, 5, ( ts_ms        & 255)::int);
    -- versión 7 en el nibble alto del byte 6
    ubytes := set_byte(ubytes, 6, ((get_byte(ubytes, 6) & 15) | 112));
    -- variante RFC 4122 (10xx) en los 2 bits altos del byte 8
    ubytes := set_byte(ubytes, 8, ((get_byte(ubytes, 8) & 63) | 128));
    RETURN encode(ubytes, 'hex')::uuid;
END;
$$ LANGUAGE plpgsql VOLATILE;

-- ---------------------------------------------------------------------
-- Tipos enumerados
-- ---------------------------------------------------------------------
DO $$ BEGIN
    CREATE TYPE estado AS ENUM ('activo', 'inactivo');
EXCEPTION WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE tipo_dispositivo AS ENUM
        ('camara', 'scanner', 'sensor', 'rfid', 'bascula', 'otro');
EXCEPTION WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE tipo_objeto AS ENUM
        ('caja', 'tarima', 'contenedor', 'producto');
EXCEPTION WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE direccion_telegrama AS ENUM ('entrada', 'salida');
EXCEPTION WHEN duplicate_object THEN null;
END $$;

-- ---------------------------------------------------------------------
-- 1. Planta
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS plantas (
    id          UUID PRIMARY KEY DEFAULT uuidv7(),
    codigo      VARCHAR(32)  NOT NULL UNIQUE,
    nombre      VARCHAR(128) NOT NULL,
    direccion   VARCHAR(255),
    estado      estado       NOT NULL DEFAULT 'activo'
);

-- ---------------------------------------------------------------------
-- 2. Area
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS areas (
    id          UUID PRIMARY KEY DEFAULT uuidv7(),
    planta_id   UUID         NOT NULL REFERENCES plantas(id) ON DELETE CASCADE,
    codigo      VARCHAR(32)  NOT NULL,
    nombre      VARCHAR(128) NOT NULL,
    descripcion TEXT
);
CREATE INDEX IF NOT EXISTS ix_areas_planta_id ON areas (planta_id);
CREATE INDEX IF NOT EXISTS ix_areas_codigo    ON areas (codigo);

-- ---------------------------------------------------------------------
-- 3. Transportador (tabla principal)
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS transportadores (
    id                UUID PRIMARY KEY DEFAULT uuidv7(),
    area_id           UUID         NOT NULL REFERENCES areas(id) ON DELETE CASCADE,
    codigo            VARCHAR(32)  NOT NULL,
    nombre            VARCHAR(128) NOT NULL,
    tipo              VARCHAR(64),
    longitud          DOUBLE PRECISION,
    ancho             DOUBLE PRECISION,
    sentido           VARCHAR(32),
    velocidad_nominal DOUBLE PRECISION,
    estado            estado       NOT NULL DEFAULT 'activo'
);
CREATE INDEX IF NOT EXISTS ix_transportadores_area_id ON transportadores (area_id);
CREATE INDEX IF NOT EXISTS ix_transportadores_codigo  ON transportadores (codigo);

-- ---------------------------------------------------------------------
-- 4. Conexión entre transportadores
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS conexiones (
    id                        UUID PRIMARY KEY DEFAULT uuidv7(),
    transportador_origen_id   UUID NOT NULL REFERENCES transportadores(id) ON DELETE CASCADE,
    transportador_destino_id  UUID NOT NULL REFERENCES transportadores(id) ON DELETE CASCADE,
    tipo_conexion             VARCHAR(64),
    distancia                 DOUBLE PRECISION,
    tiempo_estimado           DOUBLE PRECISION
);
CREATE INDEX IF NOT EXISTS ix_conexiones_origen  ON conexiones (transportador_origen_id);
CREATE INDEX IF NOT EXISTS ix_conexiones_destino ON conexiones (transportador_destino_id);

-- ---------------------------------------------------------------------
-- 5. Controlador
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS controladores (
    id            UUID PRIMARY KEY DEFAULT uuidv7(),
    codigo        VARCHAR(32)  NOT NULL UNIQUE,
    nombre        VARCHAR(128) NOT NULL,
    tipo          VARCHAR(64),
    fabricante    VARCHAR(128),
    modelo        VARCHAR(128),
    direccion_red VARCHAR(64),
    estado        estado       NOT NULL DEFAULT 'activo'
);

-- ---------------------------------------------------------------------
-- 6. Relación Controlador–Transportador (tabla intermedia)
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS controlador_transportador (
    id               UUID PRIMARY KEY DEFAULT uuidv7(),
    controlador_id   UUID NOT NULL REFERENCES controladores(id) ON DELETE CASCADE,
    transportador_id UUID NOT NULL REFERENCES transportadores(id) ON DELETE CASCADE,
    funcion          VARCHAR(64)
);
CREATE INDEX IF NOT EXISTS ix_ctrl_transp_controlador   ON controlador_transportador (controlador_id);
CREATE INDEX IF NOT EXISTS ix_ctrl_transp_transportador ON controlador_transportador (transportador_id);

-- ---------------------------------------------------------------------
-- 7. Dispositivo
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS dispositivos (
    id               UUID PRIMARY KEY DEFAULT uuidv7(),
    transportador_id UUID             NOT NULL REFERENCES transportadores(id) ON DELETE CASCADE,
    tipo             tipo_dispositivo NOT NULL DEFAULT 'otro',
    codigo           VARCHAR(32)      NOT NULL,
    nombre           VARCHAR(128)     NOT NULL,
    estado           estado           NOT NULL DEFAULT 'activo',
    ubicacion        VARCHAR(255)
);
CREATE INDEX IF NOT EXISTS ix_dispositivos_transportador ON dispositivos (transportador_id);
CREATE INDEX IF NOT EXISTS ix_dispositivos_codigo        ON dispositivos (codigo);

-- ---------------------------------------------------------------------
-- 8. Objeto
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS objetos (
    id             UUID PRIMARY KEY DEFAULT uuidv7(),
    codigo         VARCHAR(64) NOT NULL UNIQUE,
    tipo           tipo_objeto,
    estado         VARCHAR(64),
    fecha_creacion TIMESTAMP   NOT NULL DEFAULT NOW()
);

-- ---------------------------------------------------------------------
-- 9. Posición del objeto
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS posiciones (
    id               UUID PRIMARY KEY DEFAULT uuidv7(),
    objeto_id        UUID      NOT NULL REFERENCES objetos(id) ON DELETE CASCADE,
    transportador_id UUID      REFERENCES transportadores(id) ON DELETE SET NULL,
    zona             VARCHAR(64),
    fecha_hora       TIMESTAMP NOT NULL DEFAULT NOW(),
    posicion_x       DOUBLE PRECISION,
    posicion_y       DOUBLE PRECISION,
    velocidad        DOUBLE PRECISION,
    direccion        VARCHAR(32)
);
CREATE INDEX IF NOT EXISTS ix_posiciones_objeto        ON posiciones (objeto_id);
CREATE INDEX IF NOT EXISTS ix_posiciones_transportador ON posiciones (transportador_id);
CREATE INDEX IF NOT EXISTS ix_posiciones_fecha_hora    ON posiciones (fecha_hora);

-- ---------------------------------------------------------------------
-- 10. Telegrama
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS telegramas (
    id                   UUID PRIMARY KEY DEFAULT uuidv7(),
    controlador_id       UUID      REFERENCES controladores(id) ON DELETE SET NULL,
    transportador_id     UUID      REFERENCES transportadores(id) ON DELETE SET NULL,
    fecha_hora           TIMESTAMP NOT NULL DEFAULT NOW(),
    tipo                 VARCHAR(64),
    direccion            direccion_telegrama,
    contenido_original   TEXT,
    estado_procesamiento VARCHAR(64)
);
CREATE INDEX IF NOT EXISTS ix_telegramas_controlador   ON telegramas (controlador_id);
CREATE INDEX IF NOT EXISTS ix_telegramas_transportador ON telegramas (transportador_id);
CREATE INDEX IF NOT EXISTS ix_telegramas_fecha_hora    ON telegramas (fecha_hora);

-- ---------------------------------------------------------------------
-- 12. Tipo de evento
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS tipos_evento (
    id          UUID PRIMARY KEY DEFAULT uuidv7(),
    codigo      VARCHAR(32)  NOT NULL UNIQUE,
    nombre      VARCHAR(128) NOT NULL,
    descripcion TEXT
);

-- ---------------------------------------------------------------------
-- 11. Evento (todo termina aquí)
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS eventos (
    id               UUID PRIMARY KEY DEFAULT uuidv7(),
    tipo_evento_id   UUID      NOT NULL REFERENCES tipos_evento(id) ON DELETE RESTRICT,
    fecha_hora       TIMESTAMP NOT NULL DEFAULT NOW(),
    transportador_id UUID      REFERENCES transportadores(id) ON DELETE SET NULL,
    dispositivo_id   UUID      REFERENCES dispositivos(id) ON DELETE SET NULL,
    objeto_id        UUID      REFERENCES objetos(id) ON DELETE SET NULL,
    telegrama_id     UUID      REFERENCES telegramas(id) ON DELETE SET NULL,
    prioridad        INTEGER,
    estado           VARCHAR(64)
);
CREATE INDEX IF NOT EXISTS ix_eventos_tipo_evento   ON eventos (tipo_evento_id);
CREATE INDEX IF NOT EXISTS ix_eventos_transportador ON eventos (transportador_id);
CREATE INDEX IF NOT EXISTS ix_eventos_dispositivo   ON eventos (dispositivo_id);
CREATE INDEX IF NOT EXISTS ix_eventos_objeto        ON eventos (objeto_id);
CREATE INDEX IF NOT EXISTS ix_eventos_telegrama     ON eventos (telegrama_id);
CREATE INDEX IF NOT EXISTS ix_eventos_fecha_hora    ON eventos (fecha_hora);

-- ---------------------------------------------------------------------
-- Tabla de ejemplo existente (se conserva con id entero)
-- ---------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS items (
    id          SERIAL PRIMARY KEY,
    title       VARCHAR(128) NOT NULL,
    description TEXT
);
