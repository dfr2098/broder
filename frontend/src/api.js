const API_BASE = import.meta.env.VITE_API_URL || "http://localhost:8000";

export async function getItems() {
  try {
    const response = await fetch(`${API_BASE}/items`);
    return response.ok ? response.json() : [];
  } catch (error) {
    console.error("Error cargando items:", error);
    return [];
  }
}

export async function createItem(payload) {
  const response = await fetch(`${API_BASE}/items`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
  if (!response.ok) {
    throw new Error("No se pudo crear el item");
  }
  return response.json();
}
