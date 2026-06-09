FRONTEND_DIR := tron-frontend
FRONTEND_NPM := cd $(FRONTEND_DIR) && npm
BACKEND_DIR := tron-backend
BACKEND_MANIFEST := $(BACKEND_DIR)/Cargo.toml
BACKEND_ENV_FILE := $(BACKEND_DIR)/.env
BE_RUN_ENV = $(if $(strip $(PORT)),TRON_BACKEND_PORT=$(PORT)) $(if $(strip $(GRAPH_STORAGE_PATH)),TRON_BACKEND_GRAPH_STORAGE_PATH=$(GRAPH_STORAGE_PATH))

.PHONY: help
help:
	@echo "Frontend commands:"
	@echo "  make fe-install   Install frontend dependencies"
	@echo "  make fe-dev       Start the Vite dev server"
	@echo "  make fe-check     Run frontend typecheck and build"
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

.PHONY: fe-install
fe-install:
	$(FRONTEND_NPM) install

.PHONY: fe-dev
fe-dev:
	$(FRONTEND_NPM) run dev

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
	$(FRONTEND_NPM) exec playwright test $(TEST)

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
