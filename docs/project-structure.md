# 项目结构说明

```
xjy/
├── Cargo.toml              # 项目配置和依赖
├── .env                    # 环境变量 (本地开发)
├── .env.example           # 环境变量示例
│
├── docs/                  # 项目文档
│   ├── tech-stack.md      # 技术栈调研
│   ├── project-structure.md # 本文件
│   └── api.md             # API 文档 (待补充)
│
└── src/
    ├── main.rs            # 应用入口
    ├── lib.rs             # 库入口
    │
    ├── config/            # 配置管理
    │   ├── mod.rs
    │   └── database.rs    # 数据库配置
    │
    ├── models/            # 数据模型 (SeaORM entities)
    │   ├── mod.rs
    │   ├── user.rs
    │   ├── forum.rs
    │   ├── post.rs
    │   ├── comment.rs
    │   └── vote.rs
    │
    ├── handlers/          # HTTP 处理器
    │   ├── mod.rs
    │   ├── auth.rs        # 认证相关
    │   ├── forum.rs       # 板块相关
    │   ├── post.rs        # 帖子相关
    │   └── comment.rs     # 评论相关
    │
    ├── services/          # 业务逻辑层
    │   ├── mod.rs
    │   ├── auth_service.rs
    │   ├── forum_service.rs
    │   ├── post_service.rs
    │   └── comment_service.rs
    │
    ├── middleware/        # 中间件
    │   ├── mod.rs
    │   └── auth.rs        # JWT 认证中间件
    │
    ├── routes/            # 路由定义
    │   ├── mod.rs
    │   ├── auth.rs
    │   ├── forum.rs
    │   ├── post.rs
    │   └── comment.rs
    │
    ├── websocket/         # WebSocket 处理
    │   ├── mod.rs
    │   └── notification.rs
    │
    ├── error.rs           # 错误类型定义
    ├── response.rs        # 统一响应结构
    └── utils/             # 工具函数
        ├── mod.rs
        └── jwt.rs         # JWT 工具
```

## 各模块职责

### config/
- 应用配置管理
- 数据库连接池
- 环境变量加载

### models/
- SeaORM 实体定义
- 数据库表结构映射
- 数据验证逻辑

### handlers/
- HTTP 请求处理
- 参数解析和验证
- 调用 services 层

### services/
- 核心业务逻辑
- 数据库操作
- 事务处理

### middleware/
- 认证授权
- 日志记录
- 限流等

### routes/
- 路由组合
- API 版本管理
- 中间件挂载

## 分层架构原则

```
┌─────────────────────────────────┐
│      handlers (API 层)          │  ← 处理 HTTP 请求/响应
├─────────────────────────────────┤
│      services (业务层)          │  ← 业务逻辑
├─────────────────────────────────┤
│      models (数据层)            │  ← 数据模型
├─────────────────────────────────┤
│      database                   │  ← 数据存储
└─────────────────────────────────┘
```

- **handlers 不直接操作数据库**，通过 services 调用
- **services 包含可复用业务逻辑**
- **models 只定义数据结构**，不包含业务逻辑
