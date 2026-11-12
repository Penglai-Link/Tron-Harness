# AGENTS.md

## 工作流

- 目前已经发布过正式版本，需考虑API和数据兼容，不要在上一个版本的migration数据库文件上修改
- 使用 Makefile 定义的指令，完成构建、测试、运行、清理构建缓存等工作
- 如果默认端口被占用，则自行换一个可用端口再启动，同时保持前后端端口配置一致
- 注意查看tron-frontend和tron-backend里的嵌套的AGENTS.md

## 开发共识

- 后端 JSON 响应统一使用 `ApiResponse<T>` [ `tron-backend/src/response.rs` (`code`, `message`, `data`)]。每个http接口使用独立的DTO定义，不要混用
- 不要在项目内新增技术设计文档、脑爆文档、产品设计等过程文档
- 发布新的正式版都要修改CHANGELOG和推送tag，tag不要带"v"前缀


## SQLite 空间回收（改动持久化前必读）

- 业务库统一经 `framework_storage::SharedDatabase::open` 打开：设置 `auto_vacuum=INCREMENTAL`，存量库在同一个写连接上 `VACUUM` 完成一次性转换（磁盘余量不足则跳过，下次启动重试）。
- `app.rs` 启动后每 10 分钟调用 `reclaim_all_databases(1024)`，遍历进程内仍存活的数据库各回收一轮。
- 若新增 SQLite 文件绕过 `SharedDatabase`（直接 `rusqlite::Connection::open`），需自行补同样的 PRAGMA 与回收，否则文件只增不减。
