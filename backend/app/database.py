import os
from pathlib import Path

from dotenv import load_dotenv
from sqlalchemy import create_engine
from sqlalchemy.orm import declarative_base, sessionmaker

load_dotenv()

DATABASE_URL = os.getenv("DATABASE_URL") or "sqlite:///./app.db"

engine_kwargs = {"echo": False, "future": True}
if DATABASE_URL.startswith("sqlite"):
    engine_kwargs["connect_args"] = {"check_same_thread": False}

engine = create_engine(DATABASE_URL, **engine_kwargs)
SessionLocal = sessionmaker(bind=engine, autoflush=False, autocommit=False, future=True)
Base = declarative_base()

SCHEMA_PATH = Path(__file__).with_name("schema.sql")


def init_db():
    """Crea las tablas.

    En PostgreSQL (el contenedor) se usa ``schema.sql`` como fuente de
    verdad. En SQLite (desarrollo local) se usan los modelos SQLAlchemy.
    """
    from . import models  # noqa: F401  (registra los modelos en el metadata)

    if engine.dialect.name == "postgresql" and SCHEMA_PATH.exists():
        sql = SCHEMA_PATH.read_text(encoding="utf-8")
        with engine.begin() as conn:
            conn.exec_driver_sql(sql)
    else:
        Base.metadata.create_all(bind=engine)
