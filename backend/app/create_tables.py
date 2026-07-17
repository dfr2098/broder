"""Genera (crea) las tablas de la base de datos ejecutando ``schema.sql``.

El archivo ``schema.sql`` (PostgreSQL) es la fuente de verdad del esquema.
Este script lo lee y lo ejecuta contra la base de datos indicada en
``DATABASE_URL``.

Uso desde la raíz del repositorio:

    python -m backend.app.create_tables

O desde el directorio ``backend``:

    python -m app.create_tables
"""
import logging
from pathlib import Path

from sqlalchemy import inspect, text

from .database import engine

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")
logger = logging.getLogger("create_tables")

SCHEMA_PATH = Path(__file__).with_name("schema.sql")


def create_tables() -> None:
    """Ejecuta ``schema.sql`` contra la base de datos configurada.

    En PostgreSQL (el contenedor) usa ``schema.sql`` como fuente de verdad.
    En otros dialectos (ej. SQLite en desarrollo) recurre a los modelos
    SQLAlchemy, ya que el SQL está escrito para PostgreSQL.
    """
    logger.info("Generando tablas en: %s", engine.url)

    if engine.dialect.name == "postgresql":
        sql = SCHEMA_PATH.read_text(encoding="utf-8")
        # psycopg2 admite múltiples sentencias (incluidos los bloques DO $$)
        # en una sola llamada; usamos exec_driver_sql para pasarlas tal cual.
        with engine.begin() as conn:
            conn.exec_driver_sql(sql)
        with engine.connect() as conn:
            tablas = [
                r[0]
                for r in conn.execute(
                    text(
                        "SELECT table_name FROM information_schema.tables "
                        "WHERE table_schema = 'public' ORDER BY table_name"
                    )
                )
            ]
    else:
        logger.warning(
            "schema.sql está escrito para PostgreSQL; el dialecto actual es '%s'. "
            "Creando las tablas a partir de los modelos SQLAlchemy.",
            engine.dialect.name,
        )
        from .database import Base
        from . import models  # noqa: F401  (registra los modelos)

        Base.metadata.create_all(bind=engine)
        tablas = sorted(inspect(engine).get_table_names())

    if tablas:
        logger.info("Tablas disponibles: %s", ", ".join(tablas))
    logger.info("Listo.")


if __name__ == "__main__":
    create_tables()
