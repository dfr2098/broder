.PHONY: up up-docker down local create-tables vision install-backend install-frontend install-vision

up:
	@echo "Iniciando backend y frontend localmente..."
	@cd backend && python3 -m uvicorn app.main:app --host 0.0.0.0 --port 8000 --reload & \
	cd frontend && npm install && npm run dev -- --host 0.0.0.0 --port 3000

up-docker:
	docker compose --profile app up --build

down:
	docker compose --profile app --profile db --profile cache down

local:
	python3 local_worker.py

vision:
	python3 vision_worker.py

create-tables:
	python3 -m backend.app.create_tables

install-backend:
	python3 -m pip install -r backend/requirements.txt

install-frontend:
	cd frontend && npm install

install-vision:
	python3 -m pip install -r requirements-vision.txt
