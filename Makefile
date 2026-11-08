# GNU Make is used on both POSIX shells and Windows PowerShell.
# Keep recipes shell-neutral where possible and isolate the few unavoidable
# filesystem/process differences below.

ifeq ($(OS),Windows_NT)
HOST_OS := windows
SHELL := cmd.exe
.SHELLFLAGS := /D /E:ON /V:OFF /S /C
POWERSHELL := powershell.exe
# nvm-windows exposes the active Node.js version through this junction.  Use
# its absolute npm.cmd path when present so child cmd.exe processes do not
# depend on whether PATH has been refreshed after `nvm use`.
NVM_NODE_DIR := $(subst \,/,$(NVM_SYMLINK))
LOCAL_NVM_NODE_DIR := $(subst \,/,$(LOCALAPPDATA))/nodejs
NVM_NPM := $(NVM_NODE_DIR)/npm.cmd
LOCAL_NVM_NPM := $(LOCAL_NVM_NODE_DIR)/npm.cmd
NPM_BIN ?= $(if $(wildcard $(NVM_NPM)),"$(NVM_NPM)",$(if $(wildcard $(LOCAL_NVM_NPM)),"$(LOCAL_NVM_NPM)",npm))
NPM_NODE_DIR := $(if $(wildcard $(NVM_NPM)),$(NVM_NODE_DIR),$(if $(wildcard $(LOCAL_NVM_NPM)),$(LOCAL_NVM_NODE_DIR)))
ifneq ($(strip $(NPM_NODE_DIR)),)
export PATH := $(subst /,\,$(NPM_NODE_DIR));$(PATH)
endif
# npm is a .cmd batch file on Windows.  `call` is required so control returns
# to cmd.exe when npm runs inside a parenthesized or chained recipe.
NPM = call $(NPM_BIN)
CARGO := cargo.exe
PYTHON ?= python
SKILL_VALIDATOR ?= $(subst \,/,$(USERPROFILE))/.codex/skills/.system/skill-creator/scripts/quick_validate.py
# GitHub release downloads frequently time out from mainland China.  The
# variable remains overridable (including with an empty value) for other
# networks.
ELECTRON_MIRROR ?= https://registry.npmmirror.com/-/binary/electron/
# Keep local development pointed at the same persistent graph store as the
# backend runtime defaults.  Pass an explicit GRAPH_STORAGE_PATH to isolate a
# development instance when needed.
DEFAULT_GRAPH_STORAGE_PATH := $(subst \,/,$(if $(strip $(USERPROFILE)),$(USERPROFILE),$(if $(strip $(HOME)),$(HOME),~)))/.tron/tron.graph.grafeo
TRON_SCRATCH_GLOB := $(TEMP)\tron*

WIN_PATH = $(subst /,\,$(1))

define REMOVE
if exist "$(call WIN_PATH,$(1))\" rmdir /s /q "$(call WIN_PATH,$(1))"
endef

define REMOVE_FILE
if exist "$(call WIN_PATH,$(1))" del /f /q "$(call WIN_PATH,$(1))"
endef

define REMOVE_GLOB
for /d %%G in ("$(1)") do if exist "%%~G" rmdir /s /q "%%~G"
for %%G in ("$(1)") do if exist "%%~G" del /f /q "%%~G"
endef

define MKDIR
if not exist "$(call WIN_PATH,$(1))\" mkdir "$(call WIN_PATH,$(1))"
endef

define COPY_TREE
$(call MKDIR,$(2))
xcopy /e /i /h /y "$(call WIN_PATH,$(1))\*" "$(call WIN_PATH,$(2))\" >nul
endef

define COPY_FILE
copy /y "$(call WIN_PATH,$(1))" "$(call WIN_PATH,$(2))" >nul
endef

define ASSERT_FILE
if not exist "$(call WIN_PATH,$(1))" (echo Required file not found: $(1) & exit /b 1)
endef

define LOAD_ENV_FILE
if exist "$(call WIN_PATH,$(1))" (for /f "usebackq eol=# tokens=1,* delims==" %%A in ("$(call WIN_PATH,$(1))") do set "%%A=%%B")
endef

FRONTEND_API_DATA_SOURCE_ENV = $(foreach assignment,$(FRONTEND_API_DATA_SOURCE_ASSIGNMENTS),set "$(assignment)" &)
FE_API_ENV = set "VITE_TRON_API_BASE_URL=" & $(FRONTEND_API_DATA_SOURCE_ENV) set "TRON_FRONTEND_API_PROXY_TARGET=http://127.0.0.1:$(API_BACKEND_PORT)"
APP_FRONTEND_ENV = set "VITE_TRON_API_BASE_URL=" & $(FRONTEND_API_DATA_SOURCE_ENV) set "TRON_FRONTEND_OUT_DIR=$(FRONTEND_INTEGRATED_OUT_DIR)"
BE_RUN_ENV = $(if $(strip $(PORT)),set "TRON_BACKEND_PORT=$(PORT)" &) $(if $(strip $(GRAPH_STORAGE_PATH)),set "TRON_BACKEND_GRAPH_STORAGE_PATH=$(GRAPH_STORAGE_PATH)")
ELECTRON_INSTALL_ENV = $(if $(strip $(ELECTRON_MIRROR)),set "ELECTRON_MIRROR=$(ELECTRON_MIRROR)")
ELECTRON_DEV_ENV = set "TRON_ELECTRON_DEVTOOLS=1"
COMMAND_SEPARATOR := &
RUN_WITH_ENV = $(if $(strip $(1)),$(strip $(1)) & )$(2)

define RUN_IN_DIR
cd "$(call WIN_PATH,$(1))" && ($(2))
endef
else
HOST_OS := posix
SHELL := /bin/sh
.SHELLFLAGS := -eu -c
NPM := npm
CARGO := cargo
PYTHON ?= python3
SKILL_VALIDATOR ?= $(if $(strip $(CODEX_HOME)),$(CODEX_HOME),$(HOME)/.codex)/skills/.system/skill-creator/scripts/quick_validate.py
# Keep local development pointed at the same persistent graph store as the
# backend runtime defaults.  Pass an explicit GRAPH_STORAGE_PATH to isolate a
# development instance when needed.
DEFAULT_GRAPH_STORAGE_PATH := $(if $(strip $(HOME)),$(HOME),~)/.tron/tron.graph.grafeo
TRON_SCRATCH_GLOB := /tmp/tron*

define REMOVE
rm -rf -- "$(1)"
endef

define REMOVE_FILE
rm -f -- "$(1)"
endef

define REMOVE_GLOB
rm -rf -- $(1)
endef

define MKDIR
mkdir -p -- "$(1)"
endef

define COPY_TREE
cp -R "$(1)/." "$(2)/"
endef

define COPY_FILE
cp -- "$(1)" "$(2)"
endef

define ASSERT_FILE
test -f "$(1)"
endef

define LOAD_ENV_FILE
if [ -f "$(1)" ]; then set -a; . "$(1)"; set +a; fi
endef

FRONTEND_API_DATA_SOURCE_ENV = $(FRONTEND_API_DATA_SOURCE_ASSIGNMENTS)
FE_API_ENV = VITE_TRON_API_BASE_URL= $(FRONTEND_API_DATA_SOURCE_ENV) TRON_FRONTEND_API_PROXY_TARGET=http://127.0.0.1:$(API_BACKEND_PORT)
APP_FRONTEND_ENV = VITE_TRON_API_BASE_URL= $(FRONTEND_API_DATA_SOURCE_ENV) TRON_FRONTEND_OUT_DIR=$(FRONTEND_INTEGRATED_OUT_DIR)
BE_RUN_ENV = $(if $(strip $(PORT)),TRON_BACKEND_PORT=$(PORT)) $(if $(strip $(GRAPH_STORAGE_PATH)),TRON_BACKEND_GRAPH_STORAGE_PATH=$(GRAPH_STORAGE_PATH))
# Long-context tokenization frees large temporary buffers. macOS otherwise
# retains these in its large-allocation cache, growing RSS between model steps.
ifeq ($(shell uname -s),Darwin)
BE_RUN_ENV += MallocLargeCache="$${MallocLargeCache:-0}"
endif
ELECTRON_INSTALL_ENV = $(if $(strip $(ELECTRON_MIRROR)),ELECTRON_MIRROR=$(ELECTRON_MIRROR))
ELECTRON_DEV_ENV = TRON_ELECTRON_DEVTOOLS=1
COMMAND_SEPARATOR := ;
RUN_WITH_ENV = $(strip $(1)) $(2)

define RUN_IN_DIR
cd "$(1)" && $(2)
endef
endif

ELECTRON_DIR := electron
FRONTEND_DIR := tron-frontend
BACKEND_DIR := tron-backend
BACKEND_MANIFEST := $(BACKEND_DIR)/Cargo.toml
BACKEND_ENV_FILE := $(BACKEND_DIR)/.env
BACKEND_FRONTEND_DIR := $(BACKEND_DIR)/resources/frontend
FRONTEND_INTEGRATED_OUT_DIR := ../$(BACKEND_FRONTEND_DIR)
API_BACKEND_PORT := $(if $(strip $(PORT)),$(PORT),8080)
API_FRONTEND_PORT := $(if $(strip $(FE_PORT)),$(FE_PORT),5173)
API_GRAPH_STORAGE_PATH := $(if $(strip $(GRAPH_STORAGE_PATH)),$(GRAPH_STORAGE_PATH),$(DEFAULT_GRAPH_STORAGE_PATH))
FRONTEND_API_DATA_SOURCE_ASSIGNMENTS := \
	VITE_TRON_DATA_SOURCE=api \
	VITE_TRON_AUTH_DATA_SOURCE=api \
	VITE_TRON_ADMIN_DATA_SOURCE=api \
	VITE_TRON_AGENT_DATA_SOURCE=api \
	VITE_TRON_APP_DATA_SOURCE=api \
	VITE_TRON_AUTOMATION_DATA_SOURCE=api \
	VITE_TRON_CHAT_DATA_SOURCE=api \
	VITE_TRON_KNOWLEDGE_GRAPH_DATA_SOURCE=api \
	VITE_TRON_MCP_DATA_SOURCE=api \
	VITE_TRON_MEMORY_DATA_SOURCE=api \
	VITE_TRON_MODEL_DATA_SOURCE=api \
	VITE_TRON_PROMPT_DATA_SOURCE=api \
	VITE_TRON_PROJECT_DATA_SOURCE=api \
	VITE_TRON_RAG_DATA_SOURCE=api \
	VITE_TRON_RESOURCE_DATA_SOURCE=api \
	VITE_TRON_SKILL_DATA_SOURCE=api \
	VITE_TRON_TASK_DATA_SOURCE=api \
	VITE_TRON_VARIABLE_DATA_SOURCE=api

HOST_EXE_SUFFIX := $(if $(filter Windows_NT,$(OS)),.exe,)
BACKEND_RELEASE_BINARY := tron-backend$(HOST_EXE_SUFFIX)
BACKEND_RELEASE_PATH := $(BACKEND_DIR)/target/release/$(BACKEND_RELEASE_BINARY)
BACKEND_DEBUG_PATH := $(BACKEND_DIR)/target/debug/$(BACKEND_RELEASE_BINARY)
DESKTOP_SIDECAR_PATH := $(ELECTRON_DIR)/resources/binaries/$(BACKEND_RELEASE_BINARY)
ELECTRON_NPM := $(NPM)
NODE ?= node

# Compiler caches follow the current user, including Windows and Git worktrees.
_BUILD_CACHE_HOME := $(if $(filter Windows_NT,$(OS)),$(if $(strip $(USERPROFILE)),$(subst \,/,$(USERPROFILE)),$(HOME)),$(HOME))
export RUST_CACHE_DIR ?= $(_BUILD_CACHE_HOME)/rust-cache
export SCCACHE_DIR ?= $(RUST_CACHE_DIR)/sccache
export CCACHE_DIR ?= $(RUST_CACHE_DIR)/ccache

.PHONY: help
help:
	@echo "Common commands:"
	@echo "  make install    Install frontend and backend dependencies"
	@echo "  make clean      Run backend, frontend, integrated web, and desktop cleanup"
	@echo "  make version VERSION=0.2.0  Set and synchronize the product revision"
	@echo "  make version-check           Verify all product manifests use revision"
	@echo ""
	@echo "Frontend commands:"
	@echo "  make fe-install   Install frontend dependencies"
	@echo "  make fe-dev       Start the Vite dev server"
	@echo "  make fe-dev-api   Start Vite in API mode through same-origin /api proxy"
	@echo "  make fe-check     Run frontend typecheck and build"
	@echo "  make fe-unit-test Run frontend unit tests"
	@echo "  make fe-test      Run frontend Playwright browser tests"
	@echo "  make fe-build     Build the frontend"
	@echo "  make packages-build Build publishable frontend workspace packages"
	@echo "  make fe-clean     Remove frontend build artifacts"
	@echo ""
	@echo "Tron 能力包（Framework 依赖来自公开 Git 仓库）:"
	@echo "  make harness-capability-check Compile the Tron capability package"
	@echo ""
	@echo "Backend commands:"
	@echo "  make be-install   Fetch backend dependencies"
	@echo "  make be-check     Compile backend"
	@echo "  make be-clean     Remove backend build artifacts and scratch files"
	@echo "  make be-clean-package Remove backend crate artifacts; retain dependency caches"
	@echo "  make be-dev       Run backend server"
	@echo "                    Sources $(BACKEND_ENV_FILE) when present"
	@echo "                    Example: make be-dev PORT=8081"
	@echo "                    Example: make be-dev PORT=8081 GRAPH_STORAGE_PATH=/tmp/tron-8081.graph.grafeo"
	@echo "  make openapi      Generate OpenAPI YAML at openapi.yaml"
	@echo ""
	@echo "Integrated web commands:"
	@echo "  make dev-api      Start backend and frontend dev servers in API mode"
	@echo "                    Example: make dev-api PORT=8081 FE_PORT=5174"
	@echo "  make web-build    Build frontend into backend resources and compile backend"
	@echo "  make web-dev      Build integrated web and run backend server"
	@echo "                    Example: make web-dev PORT=8081"
	@echo "  make web-clean    Remove standalone and integrated frontend build artifacts"
	@echo ""
	@echo "Desktop commands:"
	@echo "  make desktop-build     Build desktop installer packages and sidecar binary"
	@echo "                         Ubuntu: AppImage、deb、rpm，自动检查并安装缺失的打包工具"
	@echo "  make desktop-test      Run Electron desktop unit tests"
	@echo "  make desktop-dev       Build desktop dependencies and run Electron dev mode"
	@echo "  make desktop-clean     Remove desktop build artifacts"
	@echo ""
	@echo "Skill commands:"
	@echo "  make skill-memory-dreaming-test"
	@echo "                    Validate the memory dreaming skill"
	@echo "  make skill-memory-spec-test"
	@echo "                    Validate the memory spec skill"
	@echo "  make skill-theme-test（主题包脚本测试）"
	@echo "构建缓存：默认当前用户目录下的 rust-cache，自动创建；可用 RUST_CACHE_DIR 覆盖"

.PHONY: _build-cache
harness-capability-check agent-capability-check agent-harness-check be-check be-dev openapi web-build web-dev desktop-build desktop-dev be-ui-acceptance: | _build-cache
_build-cache:
ifeq ($(OS),Windows_NT)
	@if not exist "$(subst /,\,$(SCCACHE_DIR))\" mkdir "$(subst /,\,$(SCCACHE_DIR))"
	@if not exist "$(subst /,\,$(CCACHE_DIR))\" mkdir "$(subst /,\,$(CCACHE_DIR))"
else
	@mkdir -p "$(SCCACHE_DIR)" "$(CCACHE_DIR)"
endif

.PHONY: clean
clean: be-clean fe-clean web-clean desktop-clean

.PHONY: install
install: fe-install be-install

.PHONY: version
version:
	$(NPM) --prefix . run version:sync -- $(if $(strip $(VERSION)),$(VERSION),$(word 2,$(MAKECMDGOALS)))

# Allow the concise form: make version 0.2.0.  GNU Make treats the version
# as another goal, so consume the positional version goal without running a
# shell command.  An empty recipe is portable between cmd.exe and POSIX sh.
%: ;

.PHONY: version-check
version-check:
	$(NPM) --prefix . run version:check

.PHONY: fe-install
fe-install:
	$(call RUN_IN_DIR,$(FRONTEND_DIR),$(NPM) install)

.PHONY: fe-dev
fe-dev:
	$(call RUN_IN_DIR,$(FRONTEND_DIR),$(NPM) run dev)

.PHONY: fe-dev-api
fe-dev-api:
	$(call RUN_IN_DIR,$(FRONTEND_DIR),$(call RUN_WITH_ENV,$(FE_API_ENV),$(NPM) run dev -- --host 0.0.0.0 --port $(API_FRONTEND_PORT)))

.PHONY: fe-build
fe-build:
	$(call RUN_IN_DIR,$(FRONTEND_DIR),$(NPM) run build)

.PHONY: packages-build
packages-build:
	$(call RUN_IN_DIR,$(FRONTEND_DIR),$(NPM) run packages:build)

.PHONY: fe-check
fe-check:
	$(call RUN_IN_DIR,$(FRONTEND_DIR),$(NPM) run build)

.PHONY: fe-test
fe-test:
	$(call RUN_IN_DIR,$(FRONTEND_DIR),$(NPM) exec playwright test -- $(TEST))

.PHONY: fe-unit-test
fe-unit-test:
	$(call RUN_IN_DIR,$(FRONTEND_DIR),$(if $(strip $(TEST)),$(NPM) exec -- cross-env NODE_OPTIONS=--no-experimental-webstorage vitest run $(TEST),$(NPM) run test:unit))

.PHONY: fe-clean
fe-clean:
	$(call REMOVE,$(FRONTEND_DIR)/dist)
	$(call REMOVE_FILE,$(FRONTEND_DIR)/node_modules/.tmp/tsconfig.node.tsbuildinfo)
	$(call REMOVE_FILE,$(FRONTEND_DIR)/tsconfig.tsbuildinfo)
	$(call REMOVE_FILE,$(FRONTEND_DIR)/vite.config.js)
	$(call REMOVE_FILE,$(FRONTEND_DIR)/vite.config.d.ts)

.PHONY: be-install
be-install:
	$(CARGO) fetch --manifest-path "$(BACKEND_MANIFEST)"

.PHONY: harness-capability-check agent-capability-check agent-harness-check

harness-capability-check:
	$(CARGO) check --manifest-path "$(BACKEND_DIR)/crates/tron-harness-capability/Cargo.toml"

# Compatibility aliases for existing local scripts.
agent-capability-check agent-harness-check: harness-capability-check

.PHONY: be-check
be-check:
	$(CARGO) check --manifest-path "$(BACKEND_MANIFEST)"

.PHONY: be-clean-package
# Retain dependency caches and running development data when reclaiming local artifacts.
be-clean-package:
	$(CARGO) clean --manifest-path "$(BACKEND_MANIFEST)" --package tron-backend

.PHONY: be-clean
ifeq ($(HOST_OS),windows)
be-clean:
	$(call REMOVE,$(BACKEND_DIR)/target)
	$(call REMOVE_GLOB,$(TRON_SCRATCH_GLOB))
else
be-clean:
	$(call REMOVE,$(BACKEND_DIR)/target)
	rm -rf /tmp/tron*
endif

.PHONY: be-dev
be-dev:
ifeq ($(HOST_OS),windows)
	$(POWERSHELL) -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command "if (-not $$env:HOME -and $$env:USERPROFILE) { $$env:HOME = $$env:USERPROFILE }; if (Test-Path -LiteralPath '$(BACKEND_ENV_FILE)') { Get-Content -LiteralPath '$(BACKEND_ENV_FILE)' | Where-Object { $$_.Trim() -and -not $$_.TrimStart().StartsWith('#') } | ForEach-Object { $$parts = $$_.Split('=', 2); if ($$parts.Length -eq 2) { [Environment]::SetEnvironmentVariable($$parts[0].Trim(), $$parts[1].Trim(), 'Process') } } }; if ('$(PORT)') { $$env:TRON_BACKEND_PORT = '$(PORT)' }; if ('$(GRAPH_STORAGE_PATH)') { $$env:TRON_BACKEND_GRAPH_STORAGE_PATH = '$(GRAPH_STORAGE_PATH)' }; & '$(CARGO)' run --manifest-path '$(BACKEND_MANIFEST)' --bin tron-backend; exit $$LASTEXITCODE"
else
	$(call LOAD_ENV_FILE,$(BACKEND_ENV_FILE)) $(COMMAND_SEPARATOR) $(call RUN_WITH_ENV,$(BE_RUN_ENV),$(CARGO) run --manifest-path "$(BACKEND_MANIFEST)" --bin tron-backend)
endif

.PHONY: openapi
openapi:
	$(CARGO) run --quiet --manifest-path "$(BACKEND_MANIFEST)" --bin export_openapi > openapi.yaml

.PHONY: dev-api
dev-api: install
ifeq ($(HOST_OS),windows)
dev-api:
	$(POWERSHELL) -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command "$$backend = Start-Process -FilePath '$(MAKE)' -ArgumentList @('be-dev','PORT=$(API_BACKEND_PORT)','GRAPH_STORAGE_PATH=$(API_GRAPH_STORAGE_PATH)') -WorkingDirectory '$(CURDIR)' -PassThru -NoNewWindow; $$frontend = $$null; try { do { if ($$backend.HasExited) { throw 'Backend exited before the health endpoint became ready.' }; Start-Sleep -Seconds 1; try { $$health = Invoke-WebRequest -UseBasicParsing -Uri 'http://127.0.0.1:$(API_BACKEND_PORT)/api/v1/health' -TimeoutSec 2 } catch { $$health = $$null } } while (-not $$health -or $$health.StatusCode -ne 200); Write-Host 'Starting frontend on http://localhost:$(API_FRONTEND_PORT) with /api proxy -> http://127.0.0.1:$(API_BACKEND_PORT)'; $$frontend = Start-Process -FilePath '$(MAKE)' -ArgumentList @('fe-dev-api','PORT=$(API_BACKEND_PORT)','FE_PORT=$(API_FRONTEND_PORT)') -WorkingDirectory '$(CURDIR)' -PassThru -NoNewWindow; Wait-Process -Id $$frontend.Id } finally { if ($$frontend -and -not $$frontend.HasExited) { Stop-Process -Id $$frontend.Id -Force }; if ($$backend -and -not $$backend.HasExited) { Stop-Process -Id $$backend.Id -Force } }"
else
dev-api:
	@set -eu; \
	backend_port='$(API_BACKEND_PORT)'; \
	frontend_port='$(API_FRONTEND_PORT)'; \
	graph_storage_path='$(API_GRAPH_STORAGE_PATH)'; \
	backend_pid=''; \
	frontend_pid=''; \
	cleanup() { test -z "$$backend_pid" || kill "$$backend_pid" >/dev/null 2>&1 || true; test -z "$$frontend_pid" || kill "$$frontend_pid" >/dev/null 2>&1 || true; }; \
	trap cleanup INT TERM EXIT; \
	$(MAKE) be-dev PORT="$$backend_port" GRAPH_STORAGE_PATH="$$graph_storage_path" & \
	backend_pid=$$!; \
	until curl -fsS "http://127.0.0.1:$$backend_port/api/v1/health" >/dev/null 2>&1; do sleep 1; done; \
	echo "Starting frontend on http://localhost:$$frontend_port with /api proxy -> http://127.0.0.1:$$backend_port"; \
	$(MAKE) fe-dev-api PORT="$$backend_port" FE_PORT="$$frontend_port" & \
	frontend_pid=$$!; \
	wait "$$backend_pid" "$$frontend_pid"
endif

.PHONY: web-build
web-build:
	$(call REMOVE,$(BACKEND_FRONTEND_DIR))
	$(call MKDIR,$(BACKEND_FRONTEND_DIR))
	$(call RUN_IN_DIR,$(FRONTEND_DIR),$(call RUN_WITH_ENV,$(APP_FRONTEND_ENV),$(NPM) run build))
	$(CARGO) build --manifest-path "$(BACKEND_MANIFEST)"

.PHONY: web-dev
web-dev:
	$(MAKE) web-build
	$(call LOAD_ENV_FILE,$(BACKEND_ENV_FILE)) $(COMMAND_SEPARATOR) $(call RUN_WITH_ENV,$(BE_RUN_ENV),$(CARGO) run --manifest-path "$(BACKEND_MANIFEST)")

.PHONY: web-clean
web-clean: fe-clean
	$(call REMOVE,$(BACKEND_FRONTEND_DIR))
	$(call MKDIR,$(BACKEND_FRONTEND_DIR))

.PHONY: _desktop-build-tools
_desktop-build-tools:
ifneq ($(HOST_OS),windows)
	@if [ "$$(uname -s)" != Linux ]; then exit 0; fi; \
	desktop_missing_packages=""; \
	for desktop_tool_package in rpmbuild:rpm xz:xz-utils unzip:unzip opj_decompress:libopenjp2-tools; do \
		desktop_tool=$${desktop_tool_package%%:*}; \
		if ! command -v "$$desktop_tool" >/dev/null 2>&1; then \
			desktop_missing_packages="$$desktop_missing_packages $${desktop_tool_package#*:}"; \
		fi; \
	done; \
	if [ -z "$$desktop_missing_packages" ]; then \
		echo "桌面打包工具检查通过，无需安装。"; exit 0; \
	fi; \
	if ! command -v apt-get >/dev/null 2>&1; then \
		echo "Linux 打包统一使用 Ubuntu；未找到 apt-get，缺少软件包：$$desktop_missing_packages" >&2; exit 1; \
	fi; \
	desktop_sudo=""; \
	if [ "$$(id -u)" != 0 ]; then \
		if ! command -v sudo >/dev/null 2>&1; then \
			echo "安装桌面打包工具需要 sudo，请先安装 sudo 或由 root 执行。" >&2; exit 1; \
		fi; \
		desktop_sudo=sudo; \
	fi; \
	echo "正在安装缺失的桌面打包工具：$$desktop_missing_packages"; \
	if ! $$desktop_sudo apt-get update; then \
		echo "APT 索引更新失败，桌面构建已停止。" >&2; exit 1; \
	fi; \
	if ! $$desktop_sudo apt-get install -y --no-install-recommends $$desktop_missing_packages; then \
		echo "桌面打包工具安装失败，桌面构建已停止。" >&2; exit 1; \
	fi; \
	for desktop_tool in rpmbuild xz unzip opj_decompress; do \
		if ! command -v "$$desktop_tool" >/dev/null 2>&1; then \
			echo "安装后仍未找到 $${desktop_tool}，桌面构建已停止。" >&2; exit 1; \
		fi; \
	done; \
	echo "桌面打包工具检查通过。"
endif

.PHONY: desktop-build
desktop-build: _desktop-build-tools
	$(MAKE) desktop-clean
	$(MAKE) web-build
ifeq ($(HOST_OS),windows)
	$(call REMOVE,$(ELECTRON_DIR)/resources/frontend)
	$(call MKDIR,$(ELECTRON_DIR)/resources/frontend)
	$(call COPY_TREE,$(BACKEND_FRONTEND_DIR),$(ELECTRON_DIR)/resources/frontend)
	$(call COPY_FILE,$(BACKEND_DIR)/application.yaml,$(ELECTRON_DIR)/resources/application.yaml)
	$(call MKDIR,$(ELECTRON_DIR)/resources/binaries)
	$(NODE) "$(ELECTRON_DIR)/scripts/write-update-config.cjs" "$(ELECTRON_DIR)/resources/update-config.json"
	$(CARGO) build --manifest-path "$(BACKEND_MANIFEST)" --release
	$(call ASSERT_FILE,$(BACKEND_RELEASE_PATH))
	$(call COPY_FILE,$(BACKEND_RELEASE_PATH),$(DESKTOP_SIDECAR_PATH))
	$(call RUN_IN_DIR,$(ELECTRON_DIR),$(call RUN_WITH_ENV,$(ELECTRON_INSTALL_ENV),$(NPM) install))
	$(call RUN_IN_DIR,$(ELECTRON_DIR),$(call RUN_WITH_ENV,$(ELECTRON_INSTALL_ENV),$(NPM) run -s ensure-electron))
	$(call RUN_IN_DIR,$(ELECTRON_DIR),$(NPM) run -s package)
else
	$(call REMOVE,$(ELECTRON_DIR)/resources/frontend)
	$(call MKDIR,$(ELECTRON_DIR)/resources/frontend)
	$(call COPY_TREE,$(BACKEND_FRONTEND_DIR),$(ELECTRON_DIR)/resources/frontend)
	cp -- "$(BACKEND_DIR)/application.yaml" "$(ELECTRON_DIR)/resources/application.yaml"
	$(call MKDIR,$(ELECTRON_DIR)/resources/binaries)
	$(NODE) "$(ELECTRON_DIR)/scripts/write-update-config.cjs" "$(ELECTRON_DIR)/resources/update-config.json"
	$(CARGO) build --manifest-path "$(BACKEND_MANIFEST)" --release
	test -f $(BACKEND_RELEASE_PATH)
	cp $(BACKEND_RELEASE_PATH) $(DESKTOP_SIDECAR_PATH)
	cd $(ELECTRON_DIR) && $(ELECTRON_INSTALL_ENV) $(ELECTRON_NPM) install
	cd $(ELECTRON_DIR) && $(ELECTRON_INSTALL_ENV) $(ELECTRON_NPM) run -s ensure-electron
	cd $(ELECTRON_DIR) && $(ELECTRON_NPM) run -s package
endif

.PHONY: desktop-dev
desktop-dev: install
	$(MAKE) web-build
	$(call REMOVE,$(ELECTRON_DIR)/resources/frontend)
	$(call MKDIR,$(ELECTRON_DIR)/resources/frontend)
	$(call COPY_TREE,$(BACKEND_FRONTEND_DIR),$(ELECTRON_DIR)/resources/frontend)
	$(call COPY_FILE,$(BACKEND_DIR)/application.yaml,$(ELECTRON_DIR)/resources/application.yaml)
	$(call MKDIR,$(ELECTRON_DIR)/resources/binaries)
	$(NODE) "$(ELECTRON_DIR)/scripts/write-update-config.cjs" "$(ELECTRON_DIR)/resources/update-config.json"
	$(CARGO) build --manifest-path "$(BACKEND_MANIFEST)" --bin tron-backend
	$(call ASSERT_FILE,$(BACKEND_DEBUG_PATH))
	$(call COPY_FILE,$(BACKEND_DEBUG_PATH),$(DESKTOP_SIDECAR_PATH))
	$(call RUN_IN_DIR,$(ELECTRON_DIR),$(call RUN_WITH_ENV,$(ELECTRON_INSTALL_ENV),$(ELECTRON_NPM) install))
	$(call RUN_IN_DIR,$(ELECTRON_DIR),$(call RUN_WITH_ENV,$(ELECTRON_INSTALL_ENV),$(ELECTRON_NPM) run -s ensure-electron))
	$(call RUN_IN_DIR,$(ELECTRON_DIR),$(call RUN_WITH_ENV,$(ELECTRON_DEV_ENV),$(NPM) run -s dev))

.PHONY: desktop-test
desktop-test:
	$(call RUN_IN_DIR,$(ELECTRON_DIR),$(NPM) test)

.PHONY: desktop-clean
desktop-clean:
	$(call REMOVE,$(ELECTRON_DIR)/dist)
	$(call REMOVE,$(ELECTRON_DIR)/resources/frontend)
	$(call REMOVE,$(ELECTRON_DIR)/resources/binaries)
	$(call REMOVE,$(ELECTRON_DIR)/resources/native)
	$(call REMOVE_FILE,$(ELECTRON_DIR)/resources/update-config.json)
	$(call REMOVE,$(ELECTRON_DIR)/node_modules)

.PHONY: skill-memory-dreaming-test
skill-memory-dreaming-test:
	$(PYTHON) "$(SKILL_VALIDATOR)" tron-backend/crates/tron-harness-capability/skills/eliteforge-memory-dreaming

.PHONY: skill-memory-spec-test
skill-memory-spec-test:
	$(PYTHON) "$(SKILL_VALIDATOR)" tron-backend/crates/tron-harness-capability/skills/eliteforge-memory-spec

.PHONY: skill-theme-test
skill-theme-test:
	$(PYTHON) "$(SKILL_VALIDATOR)" tron-backend/crates/tron-harness-capability/skills/tron-theme-spec
	$(PYTHON) -B tron-backend/crates/tron-harness-capability/skills/tron-theme-spec/scripts/test_theme_pack.py

# Dedicated real-PTY browser fixture; all state lives in TRON_ACCEPTANCE_DIRECTORY.
.PHONY: be-ui-acceptance
be-ui-acceptance:
	$(CARGO) run --manifest-path "$(BACKEND_MANIFEST)" --example ui_acceptance_server
