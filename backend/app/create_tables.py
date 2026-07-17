"""Genera (crea) las tablas de la base de datos.

Uso desde la raíz del repositorio:

    python -m backend.app.create_tables

O desde el directorio ``backend``:

    python -m app.create_tables
"""
import logging

from .database import Base, engine, init_db

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")
logger = logging.getLogger("create_tables")


def create_tables() -> None:
    """Crea todas las tablas definidas en los modelos de SQLAlchemy."""
    logger.info("Generando tablas en: %s", engine.url)
    init_db()
    tablas = sorted(Base.metadata.tables.keys())
    if tablas:
        logger.info("Tablas disponibles: %s", ", ".join(tablas))
    else:
        logger.warning("No se encontraron modelos/tablas para crear.")
    logger.info("Listo.")


if __name__ == "__main__":
    create_tables()
