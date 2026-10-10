# Tron Harness

## 概述

本项目是一个支持插件的，通用的多智能体协同工作 Harness 平台。

本项目采用 **Rust + React + Electron 单体仓库架构**，前端和后端代码位于同一个仓库中。

前端侧重于组件优先开发和强类型；后端则提供模块化、高性能的 Rust 服务。

---

## 架构概览

- **单体仓库结构**
  - 前端、后端和APP壳都位于同一个仓库中
  - 使用统一的开发工作流
  - 使用统一的构建和发布流程

- **集成式部署**
  - 前端构建为静态资源
  - 构建产物放入后端服务
  - 后端同时提供 API 端点和前端页面
  - 只需运行后端，即可访问完整应用
  - APP壳引入前后端的构建产物，不负责承担业务逻辑的代码实现

---

## 构建和运行模式

### 使用Make构建

Makefile中集成了前端、后端、WEB继承模式、桌面APP模式的相关构建和运行命令，使用 `make help` 查看使用说明。

### 构建缓存

从仓库根目录使用 Makefile 构建。后端通过 `.cargo/config.toml` 中的 `rustc-wrapper = "sccache"` 复用 Rust 编译缓存，构建环境需要提供可执行的 `sccache`。当前配置关闭了 Cargo 增量编译（`incremental = false`）。

| 缓存或产物 | 默认位置 | 复用范围 |
| --- | --- | --- |
| Rust 编译缓存 | macOS、Linux：`$HOME/rust-cache/sccache`；Windows：`%USERPROFILE%/rust-cache/sccache` | 同一用户的工作树使用同一个缓存目录；实际命中取决于源码、编译器和编译参数。 |
| C/C++ 编译缓存目录 | 用户目录下的 `rust-cache/ccache` | Makefile 创建目录并导出 `CCACHE_DIR`；是否使用缓存取决于原生依赖的编译配置，项目未统一设置 `CC`、`CXX` 包装器。 |
| Cargo 构建产物 | `tron-backend/target` | 当前后端工作树的检查、测试及 debug、release 构建产物，独立于用户级编译缓存。 |
| 前端工作区包构建缓存 | `tron-frontend/.turbo` | Turbo 缓存 `packages/*` 的构建任务，命中时恢复对应包的 `dist` 产物。 |
| TypeScript 构建信息 | `tron-frontend/tsconfig.tsbuildinfo`、`tron-frontend/node_modules/.tmp/tsconfig.node.tsbuildinfo` | 前端 TypeScript 项目构建信息。 |

后端检查、测试及包含后端构建的 Web、桌面命令会自动创建用户级缓存目录。可以覆盖缓存根目录，也可以分别指定两类编译缓存的位置：

```sh
make be-check RUST_CACHE_DIR=/path/to/rust-cache
make be-test SCCACHE_DIR=/path/to/sccache CCACHE_DIR=/path/to/ccache
```

这些变量只改变编译缓存位置，Cargo 产物仍位于各工作树的构建目录。直接运行 Cargo 时，缓存路径使用调用方的环境变量或 sccache 默认值；Makefile 的默认路径不会自动生效。

按需清理本地构建产物：

| 命令 | 清理范围 |
| --- | --- |
| `make be-clean` | 后端 `target` 和 Tron 临时文件。当前 macOS、Linux 配置会删除 `/tmp/tron*`，Windows 配置会删除 `%TEMP%` 下匹配 `tron*` 的文件和目录。 |
| `make fe-clean` | 前端应用 `dist`、上述 TypeScript 构建信息及生成的 Vite 配置文件。 |
| `make web-clean` | 执行前端清理，并重建后端用于集成前端页面的资源目录。 |
| `make desktop-clean` | 桌面打包产物、生成资源和 `electron/node_modules`；下次桌面构建会重新安装依赖。 |
| `make clean` | 执行以上全部清理。 |

这些命令会保留用户级 `sccache`、`ccache` 目录，以及前端 `.turbo` 和工作区包的 `dist`。需要清空这些缓存或包产物时，应单独处理对应目录；当前没有清空所有缓存的 Makefile 目标。用户级编译缓存由多个工作树共享，清空后各工作树都需要重新积累缓存。

### 开发模式

- 前端可以通过 Vite 开发服务器独立运行
- 后端可以通过 Cargo 独立运行
- 后端存储由后端配置选择。`TRON_BACKEND_DATABASE_URL` 将当前 SQLite 实现指向文件路径或 SQLite URI；默认 SQLite 文件位于 `$HOME/.tron/tron.sqlite3`。
- 前端组件可选使用默认数据，独立启动。不得默认使用mock数据作为API响应的兜底。
- API 集成通过 hooks 隔离
- 前端数据源选择由环境变量控制：
  - `VITE_TRON_DATA_SOURCE=mock|api` 设置全局默认值
  - `VITE_TRON_<DOMAIN>_DATA_SOURCE=mock|api` 可覆盖某个路由域，例如 `VITE_TRON_VARIABLE_DATA_SOURCE=api`
  - 页面组件必须从 `src/domain/<domain>/hooks` 导入路由域数据 hooks；端点契约保存在 `src/domain/<domain>/api` 下，静态数据样例保存在 `src/domain/<domain>/mock/data.ts` 下
  - 数据 hook 名称必须描述结果形态或访问模式，例如 `useModelList`、`useTaskDetail`、`useAppList`、`useImportApplicationDefaultsBySource`；分页结果使用 `Page`，条件读取使用 `By<Condition>`，而不是通用的 `Query` 后缀

### 生产模式

- 前端构建为静态文件
- 前端构建产物放入后端服务
- 后端提供：
  - REST 或应用 API
  - 前端静态资源
  - Web 应用入口
后端服务启动后，用户可以直接通过后端服务器访问应用界面。
