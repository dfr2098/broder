"""Modelos de datos del sistema de transportadores.

Estructura general:

    PLANTA -> AREA -> TRANSPORTADOR -> (CONEXION, DISPOSITIVO, EVENTO)
    CONTROLADOR -(CONTROLADOR_TRANSPORTADOR)- TRANSPORTADOR
    OBJETO -> POSICION -> EVENTO
    TELEGRAMA -> EVENTO
    TIPO_EVENTO -> EVENTO
"""
import enum
from datetime import datetime

from sqlalchemy import (
    Column,
    DateTime,
    Enum,
    Float,
    ForeignKey,
    Integer,
    String,
    Text,
)
from sqlalchemy.orm import relationship

from .database import Base


# ---------------------------------------------------------------------------
# Enumeraciones
# ---------------------------------------------------------------------------
class Estado(str, enum.Enum):
    activo = "activo"
    inactivo = "inactivo"


class TipoDispositivo(str, enum.Enum):
    camara = "camara"
    scanner = "scanner"
    sensor = "sensor"
    rfid = "rfid"
    bascula = "bascula"
    otro = "otro"


class TipoObjeto(str, enum.Enum):
    caja = "caja"
    tarima = "tarima"
    contenedor = "contenedor"
    producto = "producto"


class DireccionTelegrama(str, enum.Enum):
    entrada = "entrada"
    salida = "salida"


# ---------------------------------------------------------------------------
# 1. Planta
# ---------------------------------------------------------------------------
class Planta(Base):
    __tablename__ = "plantas"

    id = Column(Integer, primary_key=True, index=True)
    codigo = Column(String(32), unique=True, nullable=False, index=True)
    nombre = Column(String(128), nullable=False)
    direccion = Column(String(255), nullable=True)
    estado = Column(Enum(Estado), nullable=False, default=Estado.activo)

    areas = relationship(
        "Area", back_populates="planta", cascade="all, delete-orphan"
    )


# ---------------------------------------------------------------------------
# 2. Area
# ---------------------------------------------------------------------------
class Area(Base):
    __tablename__ = "areas"

    id = Column(Integer, primary_key=True, index=True)
    planta_id = Column(
        Integer, ForeignKey("plantas.id", ondelete="CASCADE"), nullable=False, index=True
    )
    codigo = Column(String(32), nullable=False, index=True)
    nombre = Column(String(128), nullable=False)
    descripcion = Column(Text, nullable=True)

    planta = relationship("Planta", back_populates="areas")
    transportadores = relationship(
        "Transportador", back_populates="area", cascade="all, delete-orphan"
    )


# ---------------------------------------------------------------------------
# 3. Transportador (tabla principal)
# ---------------------------------------------------------------------------
class Transportador(Base):
    __tablename__ = "transportadores"

    id = Column(Integer, primary_key=True, index=True)
    area_id = Column(
        Integer, ForeignKey("areas.id", ondelete="CASCADE"), nullable=False, index=True
    )
    codigo = Column(String(32), nullable=False, index=True)
    nombre = Column(String(128), nullable=False)
    tipo = Column(String(64), nullable=True)
    longitud = Column(Float, nullable=True)
    ancho = Column(Float, nullable=True)
    sentido = Column(String(32), nullable=True)
    velocidad_nominal = Column(Float, nullable=True)
    estado = Column(Enum(Estado), nullable=False, default=Estado.activo)

    area = relationship("Area", back_populates="transportadores")

    conexiones_origen = relationship(
        "Conexion",
        foreign_keys="Conexion.transportador_origen_id",
        back_populates="transportador_origen",
        cascade="all, delete-orphan",
    )
    conexiones_destino = relationship(
        "Conexion",
        foreign_keys="Conexion.transportador_destino_id",
        back_populates="transportador_destino",
        cascade="all, delete-orphan",
    )
    dispositivos = relationship(
        "Dispositivo", back_populates="transportador", cascade="all, delete-orphan"
    )
    eventos = relationship("Evento", back_populates="transportador")
    controladores_link = relationship(
        "ControladorTransportador",
        back_populates="transportador",
        cascade="all, delete-orphan",
    )
    telegramas = relationship("Telegrama", back_populates="transportador")
    posiciones = relationship("Posicion", back_populates="transportador")


# ---------------------------------------------------------------------------
# 4. Conexion entre transportadores
# ---------------------------------------------------------------------------
class Conexion(Base):
    __tablename__ = "conexiones"

    id = Column(Integer, primary_key=True, index=True)
    transportador_origen_id = Column(
        Integer,
        ForeignKey("transportadores.id", ondelete="CASCADE"),
        nullable=False,
        index=True,
    )
    transportador_destino_id = Column(
        Integer,
        ForeignKey("transportadores.id", ondelete="CASCADE"),
        nullable=False,
        index=True,
    )
    tipo_conexion = Column(String(64), nullable=True)
    distancia = Column(Float, nullable=True)
    tiempo_estimado = Column(Float, nullable=True)

    transportador_origen = relationship(
        "Transportador",
        foreign_keys=[transportador_origen_id],
        back_populates="conexiones_origen",
    )
    transportador_destino = relationship(
        "Transportador",
        foreign_keys=[transportador_destino_id],
        back_populates="conexiones_destino",
    )


# ---------------------------------------------------------------------------
# 5. Controlador
# ---------------------------------------------------------------------------
class Controlador(Base):
    __tablename__ = "controladores"

    id = Column(Integer, primary_key=True, index=True)
    codigo = Column(String(32), unique=True, nullable=False, index=True)
    nombre = Column(String(128), nullable=False)
    tipo = Column(String(64), nullable=True)
    fabricante = Column(String(128), nullable=True)
    modelo = Column(String(128), nullable=True)
    direccion_red = Column(String(64), nullable=True)
    estado = Column(Enum(Estado), nullable=False, default=Estado.activo)

    transportadores_link = relationship(
        "ControladorTransportador",
        back_populates="controlador",
        cascade="all, delete-orphan",
    )
    telegramas = relationship("Telegrama", back_populates="controlador")


# ---------------------------------------------------------------------------
# 6. Relacion Controlador-Transportador (tabla intermedia)
# ---------------------------------------------------------------------------
class ControladorTransportador(Base):
    __tablename__ = "controlador_transportador"

    id = Column(Integer, primary_key=True, index=True)
    controlador_id = Column(
        Integer,
        ForeignKey("controladores.id", ondelete="CASCADE"),
        nullable=False,
        index=True,
    )
    transportador_id = Column(
        Integer,
        ForeignKey("transportadores.id", ondelete="CASCADE"),
        nullable=False,
        index=True,
    )
    funcion = Column(String(64), nullable=True)

    controlador = relationship("Controlador", back_populates="transportadores_link")
    transportador = relationship("Transportador", back_populates="controladores_link")


# ---------------------------------------------------------------------------
# 7. Dispositivo
# ---------------------------------------------------------------------------
class Dispositivo(Base):
    __tablename__ = "dispositivos"

    id = Column(Integer, primary_key=True, index=True)
    transportador_id = Column(
        Integer,
        ForeignKey("transportadores.id", ondelete="CASCADE"),
        nullable=False,
        index=True,
    )
    tipo = Column(Enum(TipoDispositivo), nullable=False, default=TipoDispositivo.otro)
    codigo = Column(String(32), nullable=False, index=True)
    nombre = Column(String(128), nullable=False)
    estado = Column(Enum(Estado), nullable=False, default=Estado.activo)
    ubicacion = Column(String(255), nullable=True)

    transportador = relationship("Transportador", back_populates="dispositivos")
    eventos = relationship("Evento", back_populates="dispositivo")


# ---------------------------------------------------------------------------
# 8. Objeto
# ---------------------------------------------------------------------------
class Objeto(Base):
    __tablename__ = "objetos"

    id = Column(Integer, primary_key=True, index=True)
    codigo = Column(String(64), unique=True, nullable=False, index=True)
    tipo = Column(Enum(TipoObjeto), nullable=True)
    estado = Column(String(64), nullable=True)
    fecha_creacion = Column(DateTime, nullable=False, default=datetime.utcnow)

    posiciones = relationship(
        "Posicion", back_populates="objeto", cascade="all, delete-orphan"
    )
    eventos = relationship("Evento", back_populates="objeto")


# ---------------------------------------------------------------------------
# 9. Posicion del objeto
# ---------------------------------------------------------------------------
class Posicion(Base):
    __tablename__ = "posiciones"

    id = Column(Integer, primary_key=True, index=True)
    objeto_id = Column(
        Integer, ForeignKey("objetos.id", ondelete="CASCADE"), nullable=False, index=True
    )
    transportador_id = Column(
        Integer,
        ForeignKey("transportadores.id", ondelete="SET NULL"),
        nullable=True,
        index=True,
    )
    zona = Column(String(64), nullable=True)
    fecha_hora = Column(DateTime, nullable=False, default=datetime.utcnow, index=True)
    posicion_x = Column(Float, nullable=True)
    posicion_y = Column(Float, nullable=True)
    velocidad = Column(Float, nullable=True)
    direccion = Column(String(32), nullable=True)

    objeto = relationship("Objeto", back_populates="posiciones")
    transportador = relationship("Transportador", back_populates="posiciones")


# ---------------------------------------------------------------------------
# 10. Telegrama
# ---------------------------------------------------------------------------
class Telegrama(Base):
    __tablename__ = "telegramas"

    id = Column(Integer, primary_key=True, index=True)
    controlador_id = Column(
        Integer,
        ForeignKey("controladores.id", ondelete="SET NULL"),
        nullable=True,
        index=True,
    )
    transportador_id = Column(
        Integer,
        ForeignKey("transportadores.id", ondelete="SET NULL"),
        nullable=True,
        index=True,
    )
    fecha_hora = Column(DateTime, nullable=False, default=datetime.utcnow, index=True)
    tipo = Column(String(64), nullable=True)
    direccion = Column(Enum(DireccionTelegrama), nullable=True)
    contenido_original = Column(Text, nullable=True)
    estado_procesamiento = Column(String(64), nullable=True)

    controlador = relationship("Controlador", back_populates="telegramas")
    transportador = relationship("Transportador", back_populates="telegramas")
    eventos = relationship("Evento", back_populates="telegrama")


# ---------------------------------------------------------------------------
# 12. Tipo de evento
# ---------------------------------------------------------------------------
class TipoEvento(Base):
    __tablename__ = "tipos_evento"

    id = Column(Integer, primary_key=True, index=True)
    codigo = Column(String(32), unique=True, nullable=False, index=True)
    nombre = Column(String(128), nullable=False)
    descripcion = Column(Text, nullable=True)

    eventos = relationship("Evento", back_populates="tipo_evento")


# ---------------------------------------------------------------------------
# 11. Evento (todo termina aqui)
# ---------------------------------------------------------------------------
class Evento(Base):
    __tablename__ = "eventos"

    id = Column(Integer, primary_key=True, index=True)
    tipo_evento_id = Column(
        Integer,
        ForeignKey("tipos_evento.id", ondelete="RESTRICT"),
        nullable=False,
        index=True,
    )
    fecha_hora = Column(DateTime, nullable=False, default=datetime.utcnow, index=True)
    transportador_id = Column(
        Integer,
        ForeignKey("transportadores.id", ondelete="SET NULL"),
        nullable=True,
        index=True,
    )
    dispositivo_id = Column(
        Integer,
        ForeignKey("dispositivos.id", ondelete="SET NULL"),
        nullable=True,
        index=True,
    )
    objeto_id = Column(
        Integer, ForeignKey("objetos.id", ondelete="SET NULL"), nullable=True, index=True
    )
    telegrama_id = Column(
        Integer,
        ForeignKey("telegramas.id", ondelete="SET NULL"),
        nullable=True,
        index=True,
    )
    prioridad = Column(Integer, nullable=True)
    estado = Column(String(64), nullable=True)

    tipo_evento = relationship("TipoEvento", back_populates="eventos")
    transportador = relationship("Transportador", back_populates="eventos")
    dispositivo = relationship("Dispositivo", back_populates="eventos")
    objeto = relationship("Objeto", back_populates="eventos")
    telegrama = relationship("Telegrama", back_populates="eventos")


# ---------------------------------------------------------------------------
# Modelo existente (se conserva para no romper la app / worker actuales)
# ---------------------------------------------------------------------------
class Item(Base):
    __tablename__ = "items"

    id = Column(Integer, primary_key=True, index=True)
    title = Column(String(128), nullable=False)
    description = Column(Text, nullable=True)
