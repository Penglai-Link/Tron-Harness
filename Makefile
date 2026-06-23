FRONTEND_DIR := tron-frontend
FRONTEND_NPM := cd $(FRONTEND_DIR) && npm
BACKEND_DIR := tron-backend
BACKEND_MANIFEST := $(BACKEND_DIR)/Cargo.toml
BACKEND_ENV_FILE := $(BACKEND_DIR)/.env
BACKEND_FRONTEND_DIR := $(BACKEND_DIR)/resources/frontend
FRONTEND_INTEGRATED_OUT_DIR := ../$(BACKEND_FRONTEND_DIR)
API_BACKEND_PORT := $(if $(strip $(PORT)),$(PORT),8080)
API_FRONTEND_PORT := $(if $(strip $(FE_PORT)),$(FE_PORT),5173)
API_GRAPH_STORAGE_PATH := $(if $(strip $(GRAPH_STORAGE_PATH)),$(GRAPH_STORAGE_PATH),/tmp/tron-api-$(API_BACKEND_PORT).graph.grafeo)
BE_RUN_ENV = $(if $(strip $(PORT)),TRON_BACKEND_PORT=$(PORT)) $(if $(strip $(GRAPH_STORAGE_PATH)),TRON_BACKEND_GRAPH_STORAGE_PATH=$(GRAPH_STORAGE_PATH))
FE_API_ENV = VITE_TRON_API_BASE_URL= VITE_TRON_DATA_SOURCE=api VITE_TRON_AUTH_DATA_SOURCE=api VITE_TRON_ADMIN_DATA_SOURCE=api TRON_FRONTEND_API_PROXY_TARGET=http://127.0.0.1:$(API_BACKEND_PORT)
APP_FRONTEND_ENV = VITE_TRON_API_BASE_URL= VITE_TRON_DATA_SOURCE=api VITE_TRON_AUTH_DATA_SOURCE=api VITE_TRON_ADMIN_DATA_SOURCE=api TRON_FRONTEND_OUT_DIR=$(FRONTEND_INTEGRATED_OUT_DIR)

.PHONY: help
help:
	@echo "Common commands:"
	@echo "  make clean      Run backend, frontend, and integrated app cleanup"
	@echo ""
	@echo "Frontend commands:"
	@echo "  make fe-install   Install frontend dependencies"
	@echo "  make fe-dev       Start the Vite dev server"
	@echo "  make fe-dev-api   Start Vite in API mode through same-origin /api proxy"
	@echo "  make fe-check     Run frontend typecheck and build"
	@echo "  make fe-unit-test Run frontend unit tests"
	@echo "  make fe-test      Run frontend Playwright browser tests"
	@echo "  make fe-build     Build the frontend"
	@echo "  make fe-preview   Preview the production build"
	@echo "  make fe-clean     Remove frontend build artifacts"
	@echo ""
	@echo "Backend commands:"
	@echo "  make be-install   Fetch backend dependencies"
	@echo "  make be-check     Compile backend"
	@echo "  make be-test      Run backend tests"
	@echo "  make be-clean     Remove backend build artifacts and /tmp/tron* scratch files"
	@echo "  make be-run       Run backend server"
	@echo "                    Sources $(BACKEND_ENV_FILE) when present"
	@echo "                    Example: make be-run PORT=8081"
	@echo "                    Example: make be-run PORT=8081 GRAPH_STORAGE_PATH=/tmp/tron-8081.graph.grafeo"
	@echo ""
	@echo "Integrated app commands:"
	@echo "  make dev-api      Start backend and frontend dev servers in API mode"
	@echo "                    Example: make dev-api PORT=8081 FE_PORT=5174"
	@echo "  make app-build    Build frontend into backend resources in integrated API mode and compile backend"
	@echo "  make app-run      Build integrated app in integrated API mode and run backend server"
	@echo "                    Example: make app-run PORT=8081"
	@echo "  make app-clean    Remove standalone and integrated frontend build artifacts"
	@echo "Desktop commands:"
	@echo "  make desktop-build     Build desktop package artifacts and sidecar binary"
	@echo "  make desktop-dev       Build desktop dependencies and run Electron dev mode"
	@echo "  make desktop-clean     Remove desktop build artifacts"
	@echo ""
	@echo "Skill commands:"
	@echo "  make skill-memory-dreaming-test"
	@echo "                    Validate the memory dreaming skill"
	@echo "  make skill-memory-spec-test"
	@echo "                    Validate the memory spec skill"

.PHONY: clean
clean: be-clean fe-clean app-clean

.PHONY: fe-install
fe-install:
	$(FRONTEND_NPM) install

.PHONY: fe-dev
fe-dev:
	$(FRONTEND_NPM) run dev

.PHONY: fe-dev-api
fe-dev-api:
	cd $(FRONTEND_DIR) && $(FE_API_ENV) npm run dev -- --host 0.0.0.0 --port $(API_FRONTEND_PORT)

.PHONY: fe-build
fe-build:
	$(FRONTEND_NPM) run build

.PHONY: fe-preview
fe-preview:
	$(FRONTEND_NPM) run preview

.PHONY: fe-check
fe-check:
	$(FRONTEND_NPM) run typecheck
	$(FRONTEND_NPM) run build

.PHONY: fe-test
fe-test:
	$(FRONTEND_NPM) exec playwright test -- $(TEST)

.PHONY: fe-unit-test
fe-unit-test:
	$(FRONTEND_NPM) run test:unit

.PHONY: fe-clean
fe-clean:
	rm -rf $(FRONTEND_DIR)/dist \
		$(FRONTEND_DIR)/node_modules/.tmp/tsconfig.node.tsbuildinfo \
		$(FRONTEND_DIR)/tsconfig.tsbuildinfo \
		$(FRONTEND_DIR)/vite.config.js \
		$(FRONTEND_DIR)/vite.config.d.ts

.PHONY: be-install
be-install:
	cargo fetch --manifest-path $(BACKEND_MANIFEST)

.PHONY: be-check
be-check:
	cargo check --manifest-path $(BACKEND_MANIFEST)

.PHONY: be-test
be-test:
	cargo test --manifest-path $(BACKEND_MANIFEST)

.PHONY: be-clean
be-clean:
	rm -rf $(BACKEND_DIR)/target
	rm -rf /tmp/tron*

.PHONY: be-run
be-run:
	@if [ -f $(BACKEND_ENV_FILE) ]; then set -a; . $(BACKEND_ENV_FILE); set +a; fi; $(BE_RUN_ENV) cargo run --manifest-path $(BACKEND_MANIFEST)

.PHONY: dev-api
dev-api:
	@set -e; \
	backend_port="$(API_BACKEND_PORT)"; \
	frontend_port="$(API_FRONTEND_PORT)"; \
	graph_storage_path="$(API_GRAPH_STORAGE_PATH)"; \
	echo "Starting backend on http://127.0.0.1:$$backend_port"; \
	$(MAKE) be-run PORT=$$backend_port GRAPH_STORAGE_PATH=$$graph_storage_path & \
	backend_pid=$$!; \
	cleanup() { \
		kill $$backend_pid $$frontend_pid >/dev/null 2>&1 || true; \
	}; \
	trap cleanup INT TERM EXIT; \
	until curl -fsS "http://127.0.0.1:$$backend_port/api/v1/health" >/dev/null 2>&1; do \
		sleep 1; \
	done; \
	echo "Starting frontend on http://localhost:$$frontend_port with /api proxy -> http://127.0.0.1:$$backend_port"; \
	$(MAKE) fe-dev-api PORT=$$backend_port FE_PORT=$$frontend_port & \
	frontend_pid=$$!; \
	wait $$backend_pid $$frontend_pid

.PHONY: app-build
app-build:
	rm -rf $(BACKEND_FRONTEND_DIR)
	mkdir -p $(BACKEND_FRONTEND_DIR)
	cd $(FRONTEND_DIR) && $(APP_FRONTEND_ENV) npm run build
	cargo build --manifest-path $(BACKEND_MANIFEST)

.PHONY: app-run
app-run: app-build
	@if [ -f $(BACKEND_ENV_FILE) ]; then set -a; . $(BACKEND_ENV_FILE); set +a; fi; $(BE_RUN_ENV) cargo run --manifest-path $(BACKEND_MANIFEST)

.PHONY: app-clean
app-clean: fe-clean
	rm -rf $(BACKEND_FRONTEND_DIR)
	mkdir -p $(BACKEND_FRONTEND_DIR)

.PHONY: desktop-build
desktop-build: desktop-clean app-build
	rm -rf $(ELECTRON_DIR)/resources/frontend
	mkdir -p $(ELECTRON_DIR)/resources/frontend
	cp -R $(BACKEND_FRONTEND_DIR)/. $(ELECTRON_DIR)/resources/frontend/
	mkdir -p $(ELECTRON_DIR)/resources/binaries
	cargo build --manifest-path $(BACKEND_MANIFEST) --release
	cp tron-backend/target/release/tron-backend $(ELECTRON_DIR)/resources/binaries/tron-backend || true
	cp tron-backend/target/release/tron-backend.exe $(ELECTRON_DIR)/resources/binaries/tron-backend.exe || true
	cd $(ELECTRON_DIR) && $(ELECTRON_NPM) install
	cd $(ELECTRON_DIR) && $(ELECTRON_NPM) run -s build

.PHONY: desktop-dev
desktop-dev: desktop-build
	cd $(ELECTRON_DIR) && TRON_ELECTRON_DEVTOOLS=1 TRON_ELECTRON_TARGET_DEVTOOLS=1 $(ELECTRON_NPM) run -s dev

.PHONY: desktop-clean
desktop-clean:
	rm -rf $(ELECTRON_DIR)/dist
	rm -rf $(ELECTRON_DIR)/resources/frontend
	rm -rf $(ELECTRON_DIR)/resources/binaries

.PHONY: skill-memory-dreaming-test
skill-memory-dreaming-test:
	python3 /home/baiyihuan/.codex/skills/.system/skill-creator/scripts/quick_validate.py tron-skills/eliteforge-memory-dreaming

.PHONY: skill-memory-spec-test
skill-memory-spec-test:
	python3 /home/baiyihuan/.codex/skills/.system/skill-creator/scripts/quick_validate.py tron-skills/eliteforge-memory-spec
