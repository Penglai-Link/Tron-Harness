# Resource storage

资源附件通过 `resource.storage.profiles` 配置。业务层只保存
`storage://<profile>/<object-key>` 定位符；历史 `local://...` 定位符仍然兼容读取。

## S3-compatible storage

使用 Apache OpenDAL 的 S3 service，支持 AWS S3、MinIO 等 S3-compatible 存储。
密钥只通过环境变量注入，不写入 YAML：

```yaml
resource:
  storage:
    defaultProfile: s3-default
    profiles:
      s3-default:
        type: s3
        endpoint: https://s3.example.com
        region: us-east-1
        bucket: tron-resources
        prefix: chat-attachments
        forcePathStyle: false
        accessKeyIdEnv: AWS_ACCESS_KEY_ID
        secretAccessKeyEnv: AWS_SECRET_ACCESS_KEY
        sessionTokenEnv: AWS_SESSION_TOKEN
```

```sh
export AWS_ACCESS_KEY_ID=...
export AWS_SECRET_ACCESS_KEY=...
export AWS_SESSION_TOKEN=... # 临时凭据时设置
```

`forcePathStyle: true` 适合 MinIO 等自建服务；AWS S3 通常使用
`false`。OpenDAL 负责 SigV4、重试、上传下载和 Range 请求。服务启动时只校验
配置格式，不主动访问 bucket；首次读写时才验证远端凭据和权限。

切换默认 profile 不会自动迁移已有对象。迁移期间需要同时保留旧 local profile，
确保数据库中的历史定位符仍可读取；新上传对象才会使用新的默认 profile。
