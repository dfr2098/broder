import os
import time
import logging
from dotenv import load_dotenv
import redis
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
from backend.app.models import Item

load_dotenv()

REDIS_URL = os.getenv("REDIS_URL_HOST", "redis://localhost:6379/0")
DATABASE_URL = os.getenv("DATABASE_URL_HOST", "postgresql://appuser:password@localhost:5432/appdb")
QUEUE_KEY = "task_queue"

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")
logger = logging.getLogger("local_worker")

engine = create_engine(DATABASE_URL, future=True)
SessionLocal = sessionmaker(bind=engine, autoflush=False, autocommit=False, future=True)
redis_client = redis.Redis.from_url(REDIS_URL)


def process_task(task_data: str):
    logger.info("Procesando tarea: %s", task_data)
    parts = task_data.split("||", 1)
    title = parts[0].strip() if parts else "Tarea local"
    description = parts[1].strip() if len(parts) > 1 else "Ejecutado por local_worker.py"

    with SessionLocal() as session:
        item = Item(title=title, description=description)
        session.add(item)
        session.commit()
        logger.info("Guardado item local: %s", item.title)


def main():
    logger.info("Worker local iniciado")
    while True:
        task = redis_client.brpop(QUEUE_KEY, timeout=5)
        if task:
            _, task_data = task
            process_task(task_data.decode("utf-8"))
        else:
            logger.debug("Esperando tareas en Redis...")
        time.sleep(1)


if __name__ == "__main__":
    main()
