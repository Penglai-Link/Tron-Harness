# 前端子包发布流程

本文说明如何将 `tron-frontend/packages` 中的公共包发布到 Nexus npm 私库。

## 发布范围

以下包可以发布：

- `@tron/chat-core`
- `@tron/chat-openapi-adapter`
- `@tron/chat-agent-ui-adapter`

React、Vue、theme 和 terminal 渲染包已迁到 `eliteforge-agent-ui`，从公共项目发布 `@eliteforge/agent-ui-*`。Tron 不再发布这四个旧渲染包。

`@tron/chat-internal-adapter` 设置了 `private: true`，不会发布。

公共包通过各自 `package.json` 中的 `publishConfig` 发布到：

```text
http://nexus.cisdigital.cn/repository/npm-internal/
```

## 发布前准备

### 1. 安装依赖

先按 [接入文档](../../docs/agent-framework-ui-integration.md) 配置两个公共项目路径，再在仓库根目录执行：

```bash
make agent-deps
make agent-ui-install
make agent-ui-build
make fe-install
```

### 2. 配置 npm 认证

在 `tron-frontend/.npmrc` 中配置私库。不要将真实认证信息写入本文或提交到 Git。

```ini
registry=http://nexus.cisdigital.cn/repository/npm/
@tron:registry=http://nexus.cisdigital.cn/repository/npm-internal/
//nexus.cisdigital.cn/repository/npm-internal/:_auth=${NPM_AUTH_TOKEN}
```

`NPM_AUTH_TOKEN` 是 Nexus Basic Auth 的 Base64 值。项目已忽略 `.npmrc`，提交代码前仍应检查 Git 状态。

验证认证：

```bash
cd tron-frontend
npm whoami --registry=http://nexus.cisdigital.cn/repository/npm-internal/
```

命令应输出当前 Nexus 用户名。

## 首次发布或发布已完成版本变更的包

如果子包版本已经更新，且该版本尚未进入私库，只需构建并发布：

```bash
cd /path/to/tron
make packages-build

cd tron-frontend
npm run packages:publish
```

`packages:publish` 执行 `changeset publish`。Changesets 会比较本地版本和私库版本，只发布私库中不存在的版本。

## 后续版本发布

### 1. 记录变更

完成功能或修复后，在 `tron-frontend` 中执行：

```bash
npm run changeset
```

选择发生变更的包，并按语义化版本规则选择版本类型：

- `patch`：兼容的缺陷修复。
- `minor`：兼容的新功能。
- `major`：不兼容的变更。

提交生成的 `.changeset/*.md` 文件。

### 2. 更新版本和 CHANGELOG

准备发布时执行：

```bash
cd tron-frontend
npm run version-packages
```

该命令更新子包版本、内部依赖范围和 `CHANGELOG.md`。版本更新后不要再次执行，除非需要生成下一批版本。

### 3. 构建子包

返回仓库根目录，通过 Makefile 构建：

```bash
cd ..
make packages-build
```

该目标通过 Turbo 按依赖关系构建 `packages/*`。

### 4. 检查发布文件

发布前建议执行 dry-run：

```bash
cd tron-frontend
npm run packages:pack
```

检查每个包的文件清单，确认只包含预期的 `dist`、`README.md` 和 `package.json` 等文件。

### 5. 发布

```bash
npm run packages:publish
```

发布成功后，提交版本号和 `CHANGELOG.md` 变更。

## 验证发布结果

查询已发布版本：

```bash
npm view @tron/chat-core version \
  --registry=http://nexus.cisdigital.cn/repository/npm-internal/
```

将包名替换为其他公共包，逐一确认版本。

业务项目安装包时使用 Nexus group 仓库：

```text
http://nexus.cisdigital.cn/repository/npm/
```

## 发布失败处理

### npm 缓存权限错误

如果出现 `EPERM` 或 `~/.npm` 包含 root 所有权文件，永久修复缓存目录：

```bash
sudo chown -R "$(id -u)":"$(id -g)" "$HOME/.npm"
npm cache verify
```

无法修改用户缓存时，可以为单次发布指定临时缓存：

```bash
npm_config_cache=/private/tmp/tron-npm-publish-cache \
  npm run packages:publish
```

### 版本已存在

npm 不允许覆盖同名同版本的包。创建新的 changeset，更新版本，重新构建后再发布。

### 部分包发布成功

修复错误后重新执行：

```bash
npm run packages:publish
```

Changesets 会跳过私库中已经存在的版本，继续发布剩余包。

### 认证失败

出现 `ENEEDAUTH` 或 `E401` 时：

1. 确认命令在 `tron-frontend` 目录执行。
2. 确认 `.npmrc` 的认证条目指向 `npm-internal`。
3. 重新执行 `npm whoami`。
4. 确认 Nexus 用户拥有目标 hosted 仓库的发布权限。

不要将包发布到 `/repository/npm/`。该地址是 group/proxy 仓库，只用于安装和查询依赖。

## 发布检查清单

- [ ] 已添加并提交 changeset；已完成版本更新的首次发布除外。
- [ ] 已执行 `make packages-build`。
- [ ] dry-run 文件清单正确。
- [ ] `npm whoami` 返回预期用户。
- [ ] 公共包版本尚未存在于私库。
- [ ] `.npmrc` 未被 Git 跟踪。
- [ ] 已执行 `npm run packages:publish`。
- [ ] 已验证私库中的发布版本。
