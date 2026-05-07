FRONTEND_DIR := tron-frontend
FRONTEND_NPM := cd $(FRONTEND_DIR) && npm

.PHONY: help
help:
	@echo "Frontend commands:"
	@echo "  make fe-install   Install frontend dependencies"
	@echo "  make fe-dev       Start the Vite dev server"
	@echo "  make fe-check     Run frontend typecheck and build"
	@echo "  make fe-build     Build the frontend"
	@echo "  make fe-preview   Preview the production build"
	@echo "  make fe-clean     Remove frontend build artifacts"

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

.PHONY: fe-clean
fe-clean:
	rm -rf $(FRONTEND_DIR)/dist \
		$(FRONTEND_DIR)/tsconfig.tsbuildinfo \
		$(FRONTEND_DIR)/vite.config.js \
		$(FRONTEND_DIR)/vite.config.d.ts
