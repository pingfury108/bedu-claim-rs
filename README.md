# 百度教育自动认领工具 (bedu-claim-rs)

基于 Rust 实现的百度教育任务自动认领 CLI 工具和库，移植自 Go 版本代码。

## 功能特性

- 🚀 异步并发处理，性能高效
- 🎯 支持自定义认领参数和筛选条件
- 📊 实时显示认领进度和状态
- 🔄 自动轮询机制
- 🛡️ 用户身份验证
- 📝 详细的日志记录
- 📚 模块化设计，可作为库使用

## 项目结构

```
src/
├── api/           # API 数据结构定义
│   ├── mod.rs
│   └── types.rs
├── client/        # 客户端和认领逻辑
│   ├── mod.rs
│   ├── http.rs    # HTTP 客户端
│   └── claimer.rs # 自动认领器
├── lib.rs         # 库入口
└── main.rs        # CLI 程序入口
```

## 安装和使用

### 作为 CLI 工具使用

```bash
# 克隆项目
git clone <repository-url>
cd bedu-claim-rs

# 编译
cargo build --release

# 运行
cargo run -- --cookie "your_cookie_string"
```

### 作为库使用

在你的 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
bedu-claim = { path = "path/to/bedu-claim-rs" }
# 或从 git 安装
# bedu-claim = { git = "https://github.com/your-repo/bedu-claim-rs" }
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
```

### 库使用示例

#### 1. 基本自动认领

```rust
use bedu_claim::client::{AutoClaimer, AutoClaimConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AutoClaimConfig {
        server_base_url: "https://easylearn.baidu.com".to_string(),
        cookie: "your_cookie_here".to_string(),
        task_type: "audittask".to_string(),
        claim_limit: 10,
        interval: 3.0,
        step_id: 1,
        subject_id: 2,
        clue_type_id: 1,
    };

    let claimer = AutoClaimer::new(config);
    claimer.start().await?;

    Ok(())
}
```

#### 2. 单独使用 HTTP 客户端

```rust
use bedu_claim::client::HttpClient;
use std::collections::HashMap;
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = HttpClient::new(
        "https://easylearn.baidu.com".to_string(),
        "your_cookie_here".to_string()
    );

    // 获取用户信息
    let user_info = client.get_user_info().await?;
    println!("用户名: {}", user_info.data.user_name);

    // 获取任务列表
    let mut options = HashMap::new();
    options.insert("taskType".to_string(), json!("audittask"));
    options.insert("subject".to_string(), json!(2));

    let tasks = client.get_audit_task_list(&options).await?;
    println!("任务数量: {}", tasks.data.list.len());

    // 认领任务
    if !tasks.data.list.is_empty() {
        let task_ids = vec![tasks.data.list[0].task_id.to_string()];
        let result = client.claim_audit_task(task_ids, "audittask").await?;
        println!("认领结果: {}", result.errmsg);
    }

    Ok(())
}
```

#### 3. 手动控制认领过程

```rust
use bedu_claim::client::{AutoClaimer, AutoClaimConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AutoClaimConfig {
        server_base_url: "https://easylearn.baidu.com".to_string(),
        cookie: "your_cookie_here".to_string(),
        task_type: "producetask".to_string(),
        claim_limit: 5,
        interval: 2.0,
        step_id: 2,
        subject_id: 3,
        clue_type_id: 1,
    };

    let claimer = AutoClaimer::new(config);

    // 验证用户
    let user_name = claimer.validate_user().await?;
    println!("用户验证成功: {}", user_name);

    // 手动执行认领
    for i in 1..=3 {
        let claimed = claimer.perform_single_claim().await?;
        println!("第 {} 次认领了 {} 个任务", i, claimed);

        if claimed == 0 {
            println!("没有更多任务");
            break;
        }
    }

    Ok(())
}
```

## 使用方法

### 基本用法

```bash
cargo run -- --cookie "your_cookie_string"
```

### 完整参数

```bash
cargo run -- \
  --cookie "your_cookie_string" \
  --subject-id 2 \
  --step-id 1 \
  --clue-type-id 1 \
  --task-type audittask \
  --limit 10 \
  --interval 3.0 \
  --server https://zhiyuan.baidu.com
```

## CLI 参数说明

| 参数 | 短参数 | 默认值 | 说明 |
|------|--------|--------|------|
| `--cookie` | `-c` | 必填 | Cookie 字符串 |
| `--subject-id` | `-s` | 2 | 学科ID |
| `--step-id` | `-e` | 1 | 学段ID |
| `--clue-type-id` | `-u` | 1 | 线索类型ID |
| `--task-type` | `-t` | audittask | 任务类型 (audittask/producetask) |
| `--limit` | `-l` | 10 | 认领限制数量 |
| `--interval` | `-i` | 3.0 | 轮询间隔 (秒) |
| `--server` | | https://easylearn.baidu.com | 服务器基础URL |

## 使用示例

### 1. 默认配置认领审核任务
```bash
cargo run -- --cookie "your_cookie_here"
```

### 2. 认领数学学科的生产任务
```bash
cargo run -- \
  --cookie "your_cookie_here" \
  --subject-id 3 \
  --task-type producetask \
  --limit 20 \
  --interval 2.0
```

### 3. 高频轮询模式（1毫秒间隔）
```bash
cargo run -- \
  --cookie "your_cookie_here" \
  --interval 0.001 \
  --limit 50
```

## 日志级别

通过环境变量 `RUST_LOG` 控制日志详细程度：

```bash
# 显示所有日志
RUST_LOG=debug cargo run -- --cookie "your_cookie"

# 只显示重要信息
RUST_LOG=info cargo run -- --cookie "your_cookie"

# 只显示警告和错误
RUST_LOG=warn cargo run -- --cookie "your_cookie"
```

## 主要功能模块

### HTTP 客户端
- 自动设置浏览器 User-Agent
- 支持 Cookie 认证
- 10秒请求超时
- 错误处理和重试机制

### 自动认领逻辑
- 用户身份验证
- 任务列表获取
- 批量任务认领
- 进度跟踪和状态管理

### 数据结构
完整映射 Go 版本的所有数据结构：
- `Subject` - 学科信息
- `TaskItem` - 任务项目
- `ClaimResponse` - 认领响应
- `UserInfoResponse` - 用户信息

## 错误处理

工具会自动处理以下常见错误：
- 网络连接失败
- Cookie 失效
- API 响应错误
- JSON 解析失败

## 注意事项

1. **Cookie 获取**: 需要从浏览器开发者工具中获取有效的 Cookie
2. **参数调整**: 根据实际需求调整认领限制和轮询间隔
3. **网络环境**: 确保网络连接稳定
4. **合理使用**: 请遵守平台使用规则，避免过于频繁的请求

## 开发说明

本项目基于原 Go 代码逐行移植到 Rust，保持了原有的逻辑和功能：

- 使用 `reqwest` 作为 HTTP 客户端
- 使用 `tokio` 实现异步处理
- 使用 `clap` 进行命令行参数解析
- 使用 `serde` 进行 JSON 序列化/反序列化
- 使用 `log` 和 `env_logger` 进行日志处理

## 构建和发布

```bash
# 开发构建
cargo build

# 发布构建
cargo build --release

# 运行测试
cargo test

# 格式化代码
cargo fmt

# 代码检查
cargo clippy
```