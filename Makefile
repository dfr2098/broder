.PHONY: up up-docker down infra apps cluster-up cluster-down local run-worker install-backend install-frontend

up:
	@echo "Iniciando infraestructura y servicios de aplicación..."
	docker compose up -d --build

infra:
	docker compose -f docker-compose.infra.yml up -d

apps:
	docker compose -f docker-compose.infra.yml -f docker-compose.apps.yml up -d --build

cluster-up:
	docker compose up -d --build

cluster-down:
	docker compose down -v

up-docker:
	docker compose up -d --build

down:
	docker compose down -v

local:
	python3 local_worker.py

run-worker:
	python3 local_worker.py

install-backend:
	python3 -m pip install -r backend/requirements.txt

install-frontend:
	cd frontend && npm install
