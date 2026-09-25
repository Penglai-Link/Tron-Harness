# Tron 桌面端在线更新

Tron 的桌面端更新由 Electron 主进程负责，复用版本服务接口：

- `GET /api/v1/versions/latest?product_code=tron`
- `GET /api/v1/versions/{versionId}/download?os_type=...`

版本服务默认复用 Person-AI SDK 地址：
`https://personai.chat/person-ai-index`。如部署环境需要覆盖，可通过
`TRON_VERSION_SERVER_URL` 注入服务端根地址；更新器会自动拼接 `/api/v1`。访问令牌由已登录的前端会话临时传给 Electron 主进程，不写入磁盘。

## 本地验证

```sh
make desktop-dev

# 如需覆盖 Person-AI 默认地址
TRON_VERSION_SERVER_URL=https://index.example.com make desktop-dev
```

启动后，Tron 会在登录完成并进入受保护页面约 2.5 秒后静默检查更新；也可以在“设置 → 通用 → 版本更新”手动检查。

### 更新运行中的开发界面

桌面窗口运行时，不要清空它正在使用的前端资源目录。旧窗口仍会按旧文件名加载尚未访问过的页面，删除旧文件会导致动态模块加载失败。

仅修改前端时，可保留旧资源并构建新界面：

```sh
VITE_TRON_DATA_SOURCE=api TRON_FRONTEND_KEEP_ASSETS=1 \
  TRON_FRONTEND_OUT_DIR=../electron/resources/frontend make fe-check
```

新资源生成后，保存当前编辑内容，再刷新窗口。保留资源仅用于开发中的原地更新；正式打包仍使用 `make desktop-build` 的清理流程。

## 发布要求

服务端每个版本需要登记 `product_code=tron` 的安装包，并为当前架构登记对应的 `os_type`：

| 平台 | 架构 | `os_type` |
| --- | --- | ---: |
| Windows | x64 | 120 |
| Windows | ARM64 | 130 |
| macOS | Intel | 220 |
| macOS | Apple Silicon | 230 |

Windows 安装包由 NSIS 静默执行并自动重启。macOS 更新会打开标准 DMG，用户需要将新版本拖拽覆盖 `Applications` 中的旧版本。

正式分发时，macOS 安装包需要使用有效的签名和公证；Windows 安装包也应保持签名，否则系统可能拦截自动安装或首次启动。
