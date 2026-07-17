import os
from fastapi import FastAPI, Depends
from sqlalchemy.orm import Session
from dotenv import load_dotenv

from . import crud, models, schemas
from .database import SessionLocal, init_db

load_dotenv()

app = FastAPI(title="FastAPI Híbrido", version="1.0")
app.add_event_handler("startup", init_db)


def get_db():
    db = SessionLocal()
    try:
        yield db
    finally:
        db.close()


@app.get("/health")
def health():
    return {"status": "ok"}


@app.get("/items", response_model=list[schemas.Item])
def read_items(skip: int = 0, limit: int = 20, db: Session = Depends(get_db)):
    return crud.get_items(db, skip=skip, limit=limit)


@app.post("/items", response_model=schemas.Item, status_code=201)
def create_new_item(item: schemas.ItemCreate, db: Session = Depends(get_db)):
    return crud.create_item(db, item)
