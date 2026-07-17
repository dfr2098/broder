from typing import Optional
from pydantic import BaseModel, ConfigDict


class ItemBase(BaseModel):
    title: str
    description: Optional[str] = None


class ItemCreate(ItemBase):
    pass


class Item(ItemBase):
    id: int
    model_config = ConfigDict(from_attributes=True)
