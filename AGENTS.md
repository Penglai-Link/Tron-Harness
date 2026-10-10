# AGENTS.md

## 工作流

- 目前是开发阶段，不需考虑API和数据兼容，数据库只在同一个migration文件上修改
- 使用 Makefile 定义的指令，完成构建、测试、运行、清理构建缓存等工作
- 如果默认端口被占用，则自行换一个可用端口再启动，同时保持前后端端口配置一致
- 注意查看tron-frontend和tron-backend里的嵌套的AGENTS.md

## 开发共识

- 后端 JSON 响应统一使用 `ApiResponse<T>` [ `tron-backend/src/response.rs` (`code`, `message`, `data`)]。每个http接口使用独立的DTO定义，不要混用
- GIT管理遵循 `$eliteforge-git-feature-oriented-lite-spec`
- 不要在项目内新增技术设计文档、脑爆文档、产品设计文档
