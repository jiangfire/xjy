# XJY - Rust 论坛后端

基于 Rust + Axum + PostgreSQL 的论坛 API，目标是提供类似 Reddit / V2EX 的板块社区后端能力。

## 功能特性

- 认证与账户：注册、登录、刷新 Token、邮箱验证、忘记/重置密码、退出登录
- 内容系统：板块、帖子、评论（评论树）
- 社区互动：投票、关注、收藏、通知（REST + WebSocket）
- 反滥用：投票前置 PoW challenge（`pow_token + pow_nonce`）
- 内容组织：标签系统（公共查询 + 管理员维护）
- 审核管理：举报、管理员统计、用户角色管理、删帖删评
- 工程能力：自动迁移、Swagger/OpenAPI、限流、可选 Redis 缓存、可选 SMTP 邮件

## 技术栈

- Rust (Edition 2021), Tokio
- Axum 0.8, tower-http, tower_governor
- PostgreSQL + SeaORM + SeaORM Migration
- JWT（Access + Refresh）
- Redis（可选，连接失败时优雅降级）
- Utoipa + Swagger UI

## 项目结构

```text
xjy/
├── src/
│   ├── config/              # 配置读取
│   ├── handlers/            # HTTP 处理器
│   ├── middleware/          # 认证中间件
│   ├── migration/           # 数据库迁移
│   ├── models/              # SeaORM 模型
│   ├── routes/              # 路由注册
│   ├── services/            # 业务逻辑
│   ├── utils/               # JWT/PoW/Markdown 等工具
│   ├── websocket/           # WebSocket 通知
│   ├── main.rs              # 程序入口
│   └── lib.rs
├── tests/                   # 集成测试
├── docs/                    # 设计文档
├── uploads/                 # 上传文件目录
├── Cargo.toml
└── README.md
```

## 快速开始

### 1. 环境准备

- Rust（稳定版）
- PostgreSQL 14+
- Redis（可选）
- SMTP 服务（可选，仅用于邮件发送）

### 2. 配置环境变量

复制模板文件：

```powershell
# Windows PowerShell
Copy-Item .env.example .env
```

```bash
# Linux / macOS
cp .env.example .env
```

建议重点配置下列变量：

| 变量 | 必填 | 说明 |
| --- | --- | --- |
| `DATABASE_URL` | 是 | PostgreSQL 连接串 |
| `JWT_SECRET` | 是 | JWT 密钥，至少 32 个字符 |
| `JWT_ACCESS_EXPIRATION` | 否 | access token 秒数，默认 `900` |
| `JWT_REFRESH_EXPIRATION` | 否 | refresh token 秒数，默认 `604800` |
| `HOST` | 否 | 监听地址，默认 `127.0.0.1` |
| `PORT` | 否 | 监听端口，默认 `3000` |
| `UPLOAD_DIR` | 否 | 上传目录，默认 `./uploads` |
| `REDIS_URL` | 否 | Redis 连接串 |
| `CORS_ORIGINS` | 否 | 允许来源，`*` 或逗号分隔 |
| `REQUIRE_EMAIL_VERIFICATION` | 否 | 是否强制邮箱验证，默认 `false` |
| `POW_SECRET` | 否 | PoW 签名密钥（建议显式配置） |
| `POW_TTL_SECONDS` | 否 | PoW 有效期秒数，默认 `120` |
| `POW_DIFFICULTY` | 否 | PoW 难度，默认 `20` |
| `DB_MAX_CONNECTIONS` | 否 | 连接池最大连接数，默认 `10` |
| `DB_MIN_CONNECTIONS` | 否 | 连接池最小连接数，默认 `2` |
| `SMTP_*` | 否 | 邮件发送配置 |
| `BOOTSTRAP_ADMIN_*` | 否 | 启动时自动创建管理员 |

注意：代码读取的是 `JWT_ACCESS_EXPIRATION`；如果只设置 `JWT_EXPIRATION`，会使用默认值。

### 3. 创建数据库

```bash
createdb forum_db
```

### 4. 启动服务

```bash
cargo run
```

服务默认地址：`http://127.0.0.1:3000`

## 文档与健康检查

- 健康检查：`GET /`
- Swagger UI：`GET /swagger-ui/`
- OpenAPI JSON：`GET /api-docs/openapi.json`
- WebSocket 通知：`GET /ws?token=<jwt>`

## API 端点概览

以下为当前代码中的主要路由（前缀均为 `/api/v1`）。

### 认证（公开）

```text
POST /auth/register
POST /auth/login
POST /auth/refresh
POST /auth/verify-email
POST /auth/forgot-password
POST /auth/reset-password
```

### 认证（需登录）

```text
GET  /auth/me
POST /auth/logout
PUT  /auth/profile
PUT  /auth/password
POST /auth/resend-verification
```

### PoW（需登录）

```text
POST /pow/challenge
```

### 用户与关注

```text
GET  /users/{username}
GET  /users/{id}/followers
GET  /users/{id}/following
POST /users/{id}/follow
```

### 板块

```text
GET    /forums
GET    /forums/{slug}
POST   /forums                  # 管理员
PUT    /forums/{slug}           # 管理员
DELETE /forums/{slug}           # 管理员
```

### 帖子

```text
GET    /forums/{forum_id}/posts
GET    /posts/{id}
POST   /posts
PUT    /posts/{id}
DELETE /posts/{id}
PUT    /posts/{id}/pin          # 管理员
PUT    /posts/{id}/lock         # 管理员
```

### 评论

```text
GET    /posts/{post_id}/comments
POST   /comments
PUT    /comments/{id}
DELETE /comments/{id}
```

### 投票（需登录 + PoW）

```text
POST /posts/{id}/vote
POST /comments/{id}/vote
```

### 搜索与标签

```text
GET  /search
GET  /tags
GET  /tags/{slug}/posts
POST /admin/tags                # 管理员
PUT  /admin/tags/{id}           # 管理员
DELETE /admin/tags/{id}         # 管理员
```

### 通知

```text
GET /notifications
GET /notifications/unread-count
PUT /notifications/{id}/read
PUT /notifications/read-all
```

### 收藏

```text
POST /posts/{id}/bookmark
GET  /bookmarks
```

### 举报与审核

```text
POST /reports
GET  /admin/reports
PUT  /admin/reports/{id}/resolve
```

### 管理员

```text
GET    /admin/stats
GET    /admin/users
PUT    /admin/users/{id}/role
DELETE /admin/posts/{id}
DELETE /admin/comments/{id}
```

### 上传

```text
POST /upload/avatar
POST /upload/image
```

静态访问上传文件：`GET /uploads/{subdir}/{filename}`

## PoW 投票流程

1. 先请求 challenge：

```http
POST /api/v1/pow/challenge
Authorization: Bearer <access_token>
```

请求体示例：

```json
{
  "action": "vote",
  "target_type": "post",
  "target_id": 123
}
```

2. 客户端计算 `pow_nonce` 后，调用投票接口：

```json
{
  "value": 1,
  "pow_token": "...",
  "pow_nonce": "..."
}
```

积分规则：当前实现下，`upvote` 给内容作者 +1 分，记入 `user_points_ledger` 并汇总到 `users.karma`；删除帖子/评论时会尝试回滚相关积分。

## 响应格式

### 成功响应

```json
{
  "success": true,
  "data": {},
  "message": null
}
```

### 分页响应

```json
{
  "success": true,
  "data": {
    "items": [],
    "total": 100,
    "page": 1,
    "per_page": 20,
    "total_pages": 5
  },
  "message": null
}
```

### 错误响应

```json
{
  "error": "错误信息"
}
```

## 限流规则

- 认证路由：`5 req/s`（burst `10`）
- 公共读取路由：`30 req/s`（burst `60`）
- 需认证写入路由：`10 req/s`（burst `20`）

## 开发与测试

运行测试前请确保测试数据库可用：

```env
TEST_DATABASE_URL=postgresql://username:password@localhost:5432/forum_test_db
```

执行测试：

```bash
cargo test
```

如果本地并发测试产生互相干扰，可使用：

```bash
cargo test -- --test-threads=1
```

## 部署说明

- 启动时会自动执行数据库迁移（无需手动导入 `schema.sql`）
- 建议使用反向代理（Nginx/Caddy）并启用 HTTPS
- 生产环境请使用强随机密钥（JWT/PoW/数据库/SMTP）
- `uploads` 目录建议挂载独立持久化存储

## 参考文档

- [技术栈调研](docs/tech-stack.md)
- [项目结构说明](docs/project-structure.md)

## License

GNU Affero General Public License v3.0 (AGPL-3.0)。
详细条款见根目录 LICENSE 文件。
