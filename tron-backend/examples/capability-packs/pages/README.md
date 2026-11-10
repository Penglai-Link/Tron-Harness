# 能力包隔离页面示例

本示例提供两个可直接导入的静态页面。启用能力包后，在导航领域中选择“页面示例”。页面计数器可用于观察主题、语言切换是否保留当前页面状态。

在仓库根目录运行 `make capability-pages-example`，生成 `output/capability-pages-example.zip`，再从设置中的能力包管理导入。升级版本时修改 manifest 的 version 后重新打包；升级后需要重新启用。

## 页面和菜单声明

`manifest.json` 的 resources 必须包含 `tron/pages.json`、`tron/navigation.json` 及每个页面资源文件（resources 不接受目录）。`tron/pages.json` 使用 version=1，每个页面声明 id、title（zh-CN/en-US）、root 和 entry。页面 ID 允许字母、数字、下划线和连字符，最长 64 字符。

`tron/navigation.json` 中使用 menuType=`PackPage`，pageCode 填本包页面 ID。导入时由宿主限定包身份，不能引用其他包或替换内置页面。其他导航规则见相邻 navigation 示例。

每个页面根目录最多 256 个文件、总大小最多 16 MiB；最多声明 32 个页面。禁止符号链接、越界路径、远程资源和 CDN。HTML 最多一个脚本入口，使用 IIFE 格式；支持本地 CSS、图片和字体。不支持 ES module、动态 import、CSS @import、srcset、表单和嵌套 iframe。资源引用相对于声明它的 HTML/CSS 文件解析，不使用绝对地址、查询参数或片段。

React 作者可参考 react-source：在独立开发目录安装 React、React DOM、Vite 和 TypeScript 后，用示例 Vite 配置生成单文件 IIFE。HTML 中放置 `<div id="root"></div>` 并引用生成的 react-page.js。协议不绑定 React；开发源码不要放进 ui 资源根目录。

## 页面上下文

宿主在页面业务脚本之前注入只读 `window.tronPage`：

```js
const unsubscribe = window.tronPage.onContext(context => {
  console.log(context.packId, context.pageId, context.locale, context.appearance);
});
```

protocolVersion 为 1；context 在首次握手前为空。onContext 在已有上下文时立即回调，并返回取消订阅函数。主题颜色使用宿主推送的 `--color-*` CSS 变量，语言区域写入 document.lang。

桥接先验证来源窗口和当前实例，再建立 MessageChannel。专用通道只发送 context，不提供 API、工具、令牌、存储或业务数据。页面脚本异常会显示加载失败。页面离开、停用、更新或卸载后会销毁 iframe 和资源；切回标签页创建新实例，页面应自行处理临时状态丢失。

## HTTP 接口

- `GET /api/v1/capabilities/packs/{id}/pages`：启用包的页面目录。
- `GET /api/v1/capabilities/packs/{id}/pages/{page}`：页面描述、包版本、资源摘要及 base64 静态资源。响应使用 Tron ApiResponse，禁止缓存，由宿主 HTTP 客户端读取。

宿主用 sandbox=allow-scripts 的 srcdoc 文档加载页面。CSP 限制脚本、资源和网络请求；插件没有父页面 DOM 和登录凭据访问权。不要将 iframe 隔离视为操作系统沙箱。能力包更新事件及窗口重新获得焦点都会复核当前版本，失效页面需重新启用后打开。
