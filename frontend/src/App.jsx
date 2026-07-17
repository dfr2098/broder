import { useEffect, useState } from "react";
import { getItems, createItem } from "./api";

export default function App() {
  const [items, setItems] = useState([]);
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");

  useEffect(() => {
    fetchItems();
  }, []);

  async function fetchItems() {
    const data = await getItems();
    setItems(data || []);
  }

  async function handleSubmit(event) {
    event.preventDefault();
    if (!title) return;
    await createItem({ title, description });
    setTitle("");
    setDescription("");
    fetchItems();
  }

  return (
    <div className="app-container">
      <header>
        <h1>Proyecto Híbrido</h1>
        <p>React + FastAPI + PostgreSQL + Redis</p>
      </header>

      <section className="form-card">
        <h2>Crear item</h2>
        <form onSubmit={handleSubmit}>
          <label>
            Título
            <input
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Nombre del item"
            />
          </label>
          <label>
            Descripción
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Descripción"
            />
          </label>
          <button type="submit">Guardar</button>
        </form>
      </section>

      <section className="items-card">
        <h2>Items</h2>
        {items.length === 0 ? (
          <p>No hay items todavía.</p>
        ) : (
          <ul>
            {items.map((item) => (
              <li key={item.id}>
                <strong>{item.title}</strong>
                <p>{item.description}</p>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
