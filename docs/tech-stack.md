# Rust 论坛后端技术方案调研

> 目标：构建类似 Reddit、V2EX、Bitcoin Forum 的板块论坛网站后端

## 核心技术栈

### 1. Web 框架：Axum ⭐ 推荐

**选择理由：**
- 2025年社区首选，生态成熟
- 性能接近 Actix，但内存占用更低
- 基于 Tokio/Tower 生态，易于扩展
- 学习曲线平缓，开发效率高

**备选方案：**
- **Actix Web** - 追求极致性能时选择
- **Rocket** - 重视开发体验时选择

### 2. 数据库层：PostgreSQL + SeaORM ⭐ 推荐

**SeaORM 优势：**
- 2025年已发布 2.0 版本，生产就绪
- 异步 ORM，开发速度快
- 基于 SQLx，兼顾安全性
- 动态查询比 Diesel 更灵活

**备选方案：**
- **SQLx** - 编译时检查，无需 ORM 开销
- **Diesel** - 编译时安全优先，但泛型复杂

### 3. 认证方案

| 场景 | 推荐方案 |
|------|---------|
| 基础认证 | JWT (`jsonwebtoken`) |
| OAuth2 | [oauth2-rs](https://github.com/ramosbugs/oauth2-rs) |
| Session | Redis + `tower-sessions` |
| 现代登录 | [oauth2-passkey](https://lib.rs/crates/oauth2-passkey) (WebAuthn) |

### 4. 实时通信

论坛需要：
- **新通知推送** → WebSocket 或 SSE
- **在线状态** → Redis Pub/Sub
- **实时回复** → WebSocket (tokio-tungstenite)

## 架构设计

```
┌─────────────────────────────────────────────────────┐
│                    客户端                            │
│              (前端 SPA / 移动端)                     │
└──────────────────┬──────────────────────────────────┘
                   │ HTTPS/WebSocket
┌──────────────────▼──────────────────────────────────┐
│              Nginx 反向代理                          │
└──────────────────┬──────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────┐
│              Axum 应用层                             │
│  ┌─────────────┬─────────────┬─────────────────┐   │
│  │   REST API  │  WebSocket  │  中间件         │   │
│  │  帖子/评论   │  实时通知   │  认证/限流      │   │
│  └─────────────┴─────────────┴─────────────────┘   │
└──────┬─────────────────────────┬────────────────────┘
       │                         │
┌──────▼─────────┐     ┌────────▼────────┐
│  PostgreSQL    │     │  Redis          │
│  (SeaORM)      │     │  (会话/缓存)     │
│                │     │                 │
│  - 帖子        │     │  - Sessions     │
│  - 用户        │     │  - 在线状态     │
│  - 板块        │     │  - Pub/Sub      │
│  - 评论        │     │                 │
└────────────────┘     └─────────────────┘
```

## 核心依赖

```toml
[dependencies]
# Web 框架
axum = "0.8"
tokio = { version = "1", features = ["full"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace"] }

# 数据库
sea-orm = { version = "2.0", features = ["sqlx-postgres", "runtime-tokio-rustls"] }

# 认证
jsonwebtoken = "9"
tower-sessions = "0.13"

# WebSocket
tokio-tungstenite = "0.26"

# 序列化
serde = { version = "1", features = ["derive"] }

# 异步错误处理
anyhow = "1"
thiserror = "2"

# 日志
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

## 数据模型设计

### 核心表结构

1. **用户表 (users)**
   - id, username, email, password_hash
   - avatar_url, bio, karma, role
   - created_at, updated_at

2. **板块表 (forums)**
   - id, name, description, slug
   - sort_order, icon_url
   - created_at

3. **帖子表 (posts)**
   - id, user_id, forum_id, title, content
   - created_at, updated_at
   - upvotes, downvotes, view_count
   - is_pinned, is_locked

4. **评论表 (comments)**
   - id, post_id, user_id, parent_id
   - content, created_at, updated_at
   - upvotes, downvotes
   - path (用于嵌套评论排序)

5. **投票表 (votes)**
   - user_id, target_type, target_id, value
   - created_at

6. **通知表 (notifications)**
   - id, user_id, type, content
   - is_read, created_at

## 开发路线

### Phase 1: 基础功能
- [x] 项目初始化
- [ ] 用户注册/登录 (JWT)
- [ ] 板块列表/详情
- [ ] 帖子 CRUD
- [ ] 基础评论功能

### Phase 2: 社区功能
- [ ] 投票系统
- [ ] 嵌套评论
- [ ] 用户资料页
- [ ] 基础搜索

### Phase 3: 高级功能
- [ ] 实时通知 (WebSocket)
- [ ] 管理员面板
- [ ] 内容审核
- [ ] 缓存优化

## 参考资源

### 框架对比
- [Rust Web Frameworks Compared: Actix vs Axum vs Rocket](https://dev.to/leapcell/rust-web-frameworks-compared-actix-vs-axum-vs-rocket-4bad)
- [Actix vs Axum 讨论 (Reddit 2025)](https://www.reddit.com/r/rust/comments/1ozt50s/actixweb_vs_axum_in_20252026/)

### ORM 选择
- [SeaORM vs Diesel 对比](https://leapcell.io/blog/diesel-vs-seaorm-navigating-compile-time-vs-dynamic-orms-in-rust)
- [SeaORM 2.0 发布说明](https://www.sea-ql.org/blog/2025-09-24-sea-orm-2.0/)

### 实战项目
- [chat_server - Rust 实时聊天示例](https://github.com/abdorizak/chat_server) - 类似论坛实时功能
- [rust-backend-template - 生产级模板](https://github.com/peterkyle01/rust-backend-template)

### 认证教程
- [Using Rust and Axum to build JWT authentication](https://blog.logrocket.com/using-rust-axum-build-jwt-authentication-api/)
- [Building a Secure Rust Backend with OAuth 2.0](https://leapcell.io/blog/building-a-secure-rust-backend-with-oauth-2-0-authorization-code-flow)

## 更新日志

- 2025-01-29: 初始技术方案调研
