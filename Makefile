.PHONY: check test lint release doctor opencv-local run viewer viewer-info viewer-logs vision demo-web vision-web vision-headless vision-smoke vision-logs vision-query verify-model web-up web-down web-logs infra-up infra-down infra-logs infra-reset

-include .env

export CCACHE_DISABLE := 1
POSTGRES_DB ?= little_brother
POSTGRES_USER ?= little_brother
POSTGRES_PASSWORD ?= change-me
DB_PORT ?= 5432
DATABASE_URL = postgresql://$(POSTGRES_USER):$(POSTGRES_PASSWORD)@127.0.0.1:$(DB_PORT)/$(POSTGRES_DB)
export POSTGRES_DB POSTGRES_USER POSTGRES_PASSWORD DB_PORT DATABASE_URL

OPENCV_VERSION ?= 4.13.0
OPENCV_LOCAL_PREFIX ?= $(HOME)/.local/opencv-$(OPENCV_VERSION)
OPENCV_LOCAL_PKGCONFIG := $(OPENCV_LOCAL_PREFIX)/lib/pkgconfig
ifneq ($(wildcard $(OPENCV_LOCAL_PKGCONFIG)/opencv4.pc),)
export PKG_CONFIG_PATH := $(OPENCV_LOCAL_PKGCONFIG):$(PKG_CONFIG_PATH)
export LD_LIBRARY_PATH := $(OPENCV_LOCAL_PREFIX)/lib:$(LD_LIBRARY_PATH)
endif

DEMO_VIDEO := video prueba/WhatsApp Video 2026-07-20 at 10.07.07 AM.mp4
VIDEO ?= $(DEMO_VIDEO)
FPS ?= 5
LOG ?= logs/video-viewer.log
MODEL ?= core/yolo/models/yolo11n.onnx
CONFIDENCE ?= 0.25
NMS ?= 0.45
SOURCE_ID ?= camera-1
VISION_LOG ?= logs/vision-inference.log
SPATIAL_CONFIG ?= core/vision/config/camera-1.spatial
TRACK_MIN_HITS ?= 2
TRACK_MAX_MISSED ?= 5
TRACK_MAX_LOST_MS ?= 1500
TRACK_MIN_IOU ?= 0.05
TRACK_MAX_DISTANCE ?= 0.25
PERSISTENCE_MODE ?= required
PERSISTENCE_QUEUE ?= 256
PERSISTENCE_BATCH ?= 25
PERSISTENCE_FLUSH_MS ?= 500
WEB_BIND ?= 0.0.0.0:8081
WEB_HOST ?= 127.0.0.1
WEB_PORT ?= 8088
PERSISTENCE_ARGS = --persistence-mode "$(PERSISTENCE_MODE)" --persistence-queue "$(PERSISTENCE_QUEUE)" --persistence-batch "$(PERSISTENCE_BATCH)" --persistence-flush-ms "$(PERSISTENCE_FLUSH_MS)"
export WEB_HOST WEB_PORT

check: test lint verify-model

test:
	cd core/rs && cargo test --workspace

lint:
	cd core/rs && cargo fmt --all -- --check
	cd core/rs && cargo clippy --workspace --all-targets -- -D warnings

release:
	cd core/rs && cargo build --release --workspace

doctor:
	bash scripts/doctor.sh "$(MODEL)" "$(VIDEO)" "$(SPATIAL_CONFIG)" "$(DB_PORT)"

opencv-local:
	bash scripts/install-opencv-local.sh
	cargo clean --manifest-path core/rs/Cargo.toml

run:
	cd core/rs && cargo run -p transport-simulator

viewer:
	cargo run --manifest-path core/rs/Cargo.toml -p video-viewer -- --fps "$(FPS)" --log "$(LOG)" "$(VIDEO)"

viewer-info:
	cargo run --manifest-path core/rs/Cargo.toml -p video-viewer -- --info --fps "$(FPS)" --log "$(LOG)" "$(VIDEO)"

viewer-logs:
	tail -n 100 -F "$(LOG)"

vision:
	cargo run --manifest-path core/rs/Cargo.toml -p vision-inference -- --display --fps "$(FPS)" --confidence "$(CONFIDENCE)" --nms "$(NMS)" --source-id "$(SOURCE_ID)" --model "$(MODEL)" --spatial-config "$(SPATIAL_CONFIG)" --log "$(VISION_LOG)" --track-min-hits "$(TRACK_MIN_HITS)" --track-max-missed "$(TRACK_MAX_MISSED)" --track-max-lost-ms "$(TRACK_MAX_LOST_MS)" --track-min-iou "$(TRACK_MIN_IOU)" --track-max-distance "$(TRACK_MAX_DISTANCE)" $(PERSISTENCE_ARGS) "$(VIDEO)"

demo-web:
	$(MAKE) vision-web VIDEO="$(DEMO_VIDEO)" SOURCE_ID="camera-1" VISION_EXTRA_ARGS="--loop-video"

vision-web: web-up
	cargo run --manifest-path core/rs/Cargo.toml -p vision-inference -- --web-bind "$(WEB_BIND)" --no-persistence --fps "$(FPS)" --confidence "$(CONFIDENCE)" --nms "$(NMS)" --source-id "$(SOURCE_ID)" --model "$(MODEL)" --spatial-config "$(SPATIAL_CONFIG)" --log "$(VISION_LOG)" --track-min-hits "$(TRACK_MIN_HITS)" --track-max-missed "$(TRACK_MAX_MISSED)" --track-max-lost-ms "$(TRACK_MAX_LOST_MS)" --track-min-iou "$(TRACK_MIN_IOU)" --track-max-distance "$(TRACK_MAX_DISTANCE)" $(VISION_EXTRA_ARGS) "$(VIDEO)"

vision-headless:
	cargo run --manifest-path core/rs/Cargo.toml -p vision-inference -- --fps "$(FPS)" --confidence "$(CONFIDENCE)" --nms "$(NMS)" --source-id "$(SOURCE_ID)" --model "$(MODEL)" --spatial-config "$(SPATIAL_CONFIG)" --log "$(VISION_LOG)" --track-min-hits "$(TRACK_MIN_HITS)" --track-max-missed "$(TRACK_MAX_MISSED)" --track-max-lost-ms "$(TRACK_MAX_LOST_MS)" --track-min-iou "$(TRACK_MIN_IOU)" --track-max-distance "$(TRACK_MAX_DISTANCE)" $(PERSISTENCE_ARGS) "$(VIDEO)"

vision-smoke:
	cargo run --manifest-path core/rs/Cargo.toml -p vision-inference -- --fps "$(FPS)" --confidence "$(CONFIDENCE)" --nms "$(NMS)" --source-id "$(SOURCE_ID)" --model "$(MODEL)" --spatial-config "$(SPATIAL_CONFIG)" --log "$(VISION_LOG)" --track-min-hits "$(TRACK_MIN_HITS)" --track-max-missed "$(TRACK_MAX_MISSED)" --track-max-lost-ms "$(TRACK_MAX_LOST_MS)" --track-min-iou "$(TRACK_MIN_IOU)" --track-max-distance "$(TRACK_MAX_DISTANCE)" $(PERSISTENCE_ARGS) --max-inferences 6 "$(VIDEO)"

vision-logs:
	tail -n 100 -F "$(VISION_LOG)"

vision-query:
	docker compose exec -T db psql -U "$${POSTGRES_USER:-little_brother}" -d "$${POSTGRES_DB:-little_brother}" -c "SELECT * FROM temporal.vision_detection ORDER BY occurred_at DESC LIMIT 20;"

verify-model:
	cd core/yolo/models && sha256sum -c SHA256SUMS

web-up:
	@if docker compose ps --status running --services | grep -qx web; then \
		echo "little-brother-web ya está ejecutándose; se reutiliza el contenedor"; \
	else \
		docker compose up -d --build web; \
	fi

web-down:
	docker compose stop web

web-logs:
	docker compose logs -f web

infra-up:
	docker compose up -d db

infra-down:
	docker compose down

infra-logs:
	docker compose logs -f db

infra-reset:
	docker compose down -v
