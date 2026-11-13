# 前端子包发布流程

本文说明如何将 `tron-frontend/packages` 中的公共包发布到 npm registry。

## 发布范围

以下包可以发布：

- `@tron/chat-core`
- `@tron/chat-openapi-adapter`
- `@tron/chat-agent-ui-adapter`

React、Vue、theme 和 terminal 渲染包已迁到 `eliteforge-agent-ui`，从公共项目发布 `@personai/agent-ui-*`。Tron 不再发布这四个旧渲染包。

`@tron/chat-internal-adapter` 设置了 `private: true`，不会发布。

发布目标 registry 由 `tron-frontend/.npmrc` 或 `npm publish --registry` 决定，未配置时使用 npm 默认的公共 registry：

```text
https://registry.npmjs.org/
```

## 发布前准备

### 1. 安装依赖

Agent UI 从公共 npm 安装。在仓库根目录执行：

```bash
make fe-install
```

### 2. 配置 npm 认证

在 `tron-frontend/.npmrc` 中配置 registry 与认证。不要将真实 token 写入本文或提交到 Git。

```ini
registry=https://registry.npmjs.org/
@tron:registry=https://registry.npmjs.org/
@personai:registry=https://registry.npmjs.org/
//registry.npmjs.org/:_authToken=${NPM_TOKEN}
```

`NPM_TOKEN` 是该 registry 的认证 token；使用私有 registry 时，把地址和认证条目替换为团队配置。项目已忽略 `.npmrc`，提交代码前仍应检查 Git 状态。

验证认证：

```bash
cd tron-frontend
npm whoami
```

命令应输出当前 registry 用户名。

## 首次发布或发布已完成版本变更的包

如果子包版本已经更新，且该版本尚未进入 registry，只需构建并发布：

```bash
cd /path/to/tron
make packages-build

cd tron-frontend
npm run packages:publish
```

`packages:publish` 执行 `changeset publish`。Changesets 会比较本地版本和 registry 版本，只发布 registry 中不存在的版本。

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
npm view @tron/chat-core version
```

将包名替换为其他公共包，逐一确认版本。

业务项目安装包时使用同一 registry：

```text
https://registry.npmjs.org/
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

Changesets 会跳过 registry 中已经存在的版本，继续发布剩余包。

### 认证失败

出现 `ENEEDAUTH` 或 `E401` 时：

1. 确认命令在 `tron-frontend` 目录执行。
2. 确认 `.npmrc` 的 registry 与 token 配置正确。
3. 重新执行 `npm whoami`。
4. 确认 token 具有目标 registry 的发布权限。

## 发布检查清单

- [ ] 已添加并提交 changeset；已完成版本更新的首次发布除外。
- [ ] 已执行 `make packages-build`。
- [ ] dry-run 文件清单正确。
- [ ] `npm whoami` 返回预期用户。
- [ ] 公共包版本尚未存在于 registry。
- [ ] `.npmrc` 未被 Git 跟踪。
- [ ] 已执行 `npm run packages:publish`。
- [ ] 已验证 registry 中的发布版本。
