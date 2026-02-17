# XJY - Rust 论坛后端

基于 Rust 的论坛后端 API，类似 Reddit、V2EX、Bitcoin Forum 的板块论坛系统。

## 技术栈

- **Web 框架**: Axum 0.8
- **数据库**: PostgreSQL + SeaORM
- **认证**: JWT (jsonwebtoken)
- **异步运行时**: Tokio
- **缓存**: Redis (可选，优雅降级)
- **WebSocket**: Axum WS (实时通知)
- **全文搜索**: PostgreSQL tsvector + GIN 索引
- **限流**: tower_governor (基于 IP 的三级限流)
- **文件上传**: Multipart + 本地存储

## 项目结构

```
xjy/
├── src/
│   ├── main.rs              # 应用入口
│   ├── lib.rs               # 库入口
│   ├── config/              # 配置管理
│   ├── models/              # 数据模型 (SeaORM)
│   ├── handlers/            # HTTP 处理器
│   ├── services/            # 业务逻辑层
│   ├── middleware/          # 中间件
│   ├── migration/           # 数据库迁移
│   ├── routes/              # 路由定义
│   ├── websocket/           # WebSocket 处理
│   ├── error.rs             # 错误类型
│   ├── response.rs          # 统一响应结构
│   └── utils/               # 工具函数
├── docs/                    # 项目文档
└── .env.example             # 环境变量示例
```

## 快速开始

### 1. 环境准备

- 安装 [Rust](https://www.rust-lang.org/tools/install) (1.70+)
- 安装 [PostgreSQL](https://www.postgresql.org/download/) (14+)

### 2. 配置环境变量

复制 `.env.example` 到 `.env` 并修改配置：

```bash
cp .env.example .env
```

编辑 `.env` 文件：

⚠️ **安全警告**: 生产环境必须使用安全的密钥，不要使用示例值。

```bash
# 生成安全的 JWT 密钥
openssl rand -hex 32
```

```env
DATABASE_URL=postgresql://<username>:<password>@localhost:5432/forum_db
JWT_SECRET=<使用上面命令生成>
REDIS_URL=redis://localhost:6379
UPLOAD_DIR=./uploads
HOST=127.0.0.1
PORT=3000
RUST_LOG=info
```

### 3. 创建数据库

```bash
createdb forum_db
# 迁移会在启动时自动执行
```

### 4. 运行项目

```bash
# 开发模式
cargo run

# 生产构建
cargo build --release
./target/release/xjy
```

服务将在 http://127.0.0.1:3000 启动。

## API 端点

### 健康检查

```
GET /
```

### 认证

```
POST /api/v1/auth/register          # 用户注册 (返回验证 token)
POST /api/v1/auth/login             # 用户登录
POST /api/v1/auth/verify-email      # 邮箱验证 (公开, 基于 token)
GET  /api/v1/auth/me                # 获取当前用户 (需认证)
PUT  /api/v1/auth/profile           # 更新个人资料 (需认证)
PUT  /api/v1/auth/password          # 修改密码 (需认证)
POST /api/v1/auth/resend-verification  # 重新发送验证 (需认证)
```

### 用户

```
GET  /api/v1/users/:username        # 获取用户公开资料
GET  /api/v1/users/:id/followers    # 获取粉丝列表 (分页)
GET  /api/v1/users/:id/following    # 获取关注列表 (分页)
POST /api/v1/users/:id/follow       # 关注/取关 (需认证)
```

### 板块

```
GET    /api/v1/forums         # 获取板块列表
GET    /api/v1/forums/:slug   # 获取板块详情
POST   /api/v1/forums         # 创建板块 (管理员)
PUT    /api/v1/forums/:slug   # 更新板块 (管理员)
DELETE /api/v1/forums/:slug   # 删除板块 (管理员)
```

### 帖子

```
GET    /api/v1/forums/:forum_id/posts  # 获取板块帖子列表 (分页)
GET    /api/v1/posts/:id               # 获取帖子详情
POST   /api/v1/posts                   # 创建帖子 (需认证)
PUT    /api/v1/posts/:id               # 更新帖子 (作者)
DELETE /api/v1/posts/:id               # 删除帖子 (作者)
PUT    /api/v1/posts/:id/pin           # 置顶/取消置顶 (管理员)
PUT    /api/v1/posts/:id/lock          # 锁定/解锁 (管理员)
POST   /api/v1/posts/:id/vote          # 投票 (需认证)
```

### 评论

```
GET    /api/v1/posts/:post_id/comments  # 获取评论树 (嵌套结构)
POST   /api/v1/comments                 # 创建评论 (需认证, 支持 parent_id)
PUT    /api/v1/comments/:id             # 更新评论 (作者)
DELETE /api/v1/comments/:id             # 删除评论 (作者)
POST   /api/v1/comments/:id/vote        # 投票 (需认证)
```

### 搜索

```
GET  /api/v1/search?q=...&forum_id=...&page=...&per_page=...  # 全文搜索帖子
```

### 通知

```
GET  /api/v1/notifications                  # 获取通知列表 (需认证, 分页)
GET  /api/v1/notifications/unread-count     # 获取未读数量 (需认证)
PUT  /api/v1/notifications/:id/read         # 标记已读 (需认证)
PUT  /api/v1/notifications/read-all         # 全部标记已读 (需认证)
```

### 举报

```
POST /api/v1/reports                        # 创建举报 (需认证)
```

### 收藏

```
POST /api/v1/posts/:id/bookmark             # 收藏/取消收藏 (需认证)
GET  /api/v1/bookmarks                      # 获取收藏列表 (需认证, 分页)
```

### 文件上传

```
POST /api/v1/upload/avatar                  # 上传头像 (需认证, multipart)
POST /api/v1/upload/image                   # 上传图片 (需认证, multipart)
GET  /uploads/{subdir}/{filename}           # 访问上传文件 (静态服务)
```

### 管理员

```
GET    /api/v1/admin/stats                  # 系统统计
GET    /api/v1/admin/users?page=&per_page=  # 用户列表
PUT    /api/v1/admin/users/:id/role         # 修改用户角色
DELETE /api/v1/admin/posts/:id              # 删除帖子
DELETE /api/v1/admin/comments/:id           # 删除评论
GET    /api/v1/admin/reports?status=&page=  # 举报列表
PUT    /api/v1/admin/reports/:id/resolve    # 处理举报 (hide/delete/dismiss)
```

### WebSocket

```
GET  /ws?token=<jwt>  # 实时通知推送
```

## 开发状态

### Phase 1: 基础功能
- [x] 项目初始化
- [x] 基础框架搭建
- [x] 错误处理
- [x] JWT 工具
- [x] 用户注册/登录
- [x] 板块 CRUD
- [x] 帖子 CRUD
- [x] 评论系统
- [x] 数据库迁移 (自动运行)

### Phase 2: 社区功能
- [x] 投票系统
- [x] 用户资料页
- [x] 帖子浏览计数
- [x] 管理员帖子管理 (置顶/锁定)
- [x] 板块更新/删除
- [x] 嵌套评论渲染 (树形结构, 深度限制 10 层)
- [x] 全文搜索 (PostgreSQL tsvector + GIN 索引)

### Phase 3: 高级功能
- [x] 实时通知 (WebSocket + REST API)
- [x] 管理员面板 (统计/用户管理/内容管理)
- [x] 内容审核 (举报系统 + 隐藏/删除/驳回)
- [x] 缓存优化 (Redis, 可选, 优雅降级)

### Phase 4: 用户功能 & 基础设施
- [x] 通用分页响应 (`PaginatedResponse<T>`)
- [x] 修改密码
- [x] 邮箱验证 (token 模式)
- [x] 收藏系统 (toggle + 列表)
- [x] 关注系统 (toggle + 粉丝/关注列表)
- [x] 图片/文件上传 (Multipart, 本地存储, 静态服务)
- [x] 限流 (tower_governor, 三级: auth 5/s, 读 30/s, 写 10/s)

## API 文档

### 认证流程

所有需要认证的端点需要在请求头中包含 JWT token：
```
Authorization: Bearer <access_token>
```

### 请求/响应格式

**成功响应**:
```json
{
  "data": { ... },
  "total": 100,
  "page": 1,
  "limit": 20
}
```

**错误响应**:
```json
{
  "error": "错误信息"
}
```

### 错误码

| 状态码 | 说明 |
|--------|------|
| 400 | 请求参数错误 |
| 401 | 未认证或 token 无效 |
| 403 | 权限不足 |
| 404 | 资源不存在 |
| 409 | 资源冲突 |
| 429 | 请求过于频繁 |
| 500 | 服务器错误 |

### 限流规则

- 认证端点: 5 req/s
- 读取端点: 30 req/s
- 写入端点: 10 req/s

## 安全最佳实践

- **密钥生成**: 使用 `openssl rand -hex 32` 生成 JWT 密钥
- **HTTPS**: 生产环境必须使用 HTTPS (推荐 Let's Encrypt)
- **文件上传**: 验证文件类型和大小，防止目录遍历
- **环境变量**: 永远不要提交 `.env` 文件到版本控制
- **密钥轮换**: 定期更换密钥和凭证
- **数据库**: 生产环境启用 PostgreSQL SSL 连接

## 部署指南

### 生产环境配置

**环境变量**:
```env
DATABASE_URL=postgresql://<user>:<pass>@host:5432/db
JWT_SECRET=<使用 openssl rand -hex 32 生成>
REDIS_URL=redis://host:6379
UPLOAD_DIR=/var/www/uploads
HOST=0.0.0.0
PORT=3000
RUST_LOG=info
```

**数据库设置**:
```bash
createdb forum_db
psql forum_db < schema.sql  # 迁移自动运行
```

**Systemd 服务** (`/etc/systemd/system/xjy.service`):
```ini
[Unit]
Description=XJY Forum API
After=network.target postgresql.service

[Service]
Type=simple
User=www-data
WorkingDirectory=/opt/xjy
EnvironmentFile=/opt/xjy/.env
ExecStart=/opt/xjy/target/release/xjy
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

**Nginx 反向代理** (生产环境需配置 HTTPS):
```nginx
server {
    listen 443 ssl http2;
    server_name api.example.com;

    ssl_certificate /etc/letsencrypt/live/api.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.example.com/privkey.pem;

    # 安全头
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    location /uploads/ {
        alias /var/www/uploads/;
    }
}

server {
    listen 80;
    server_name api.example.com;
    return 301 https://$server_name$request_uri;
}
```

## 文档

- [技术栈调研](docs/tech-stack.md)
- [项目结构说明](docs/project-structure.md)

## License

MIT
