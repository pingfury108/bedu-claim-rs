//! # Bedu Claim - 百度教育自动认领工具
//!
//! 这是一个用于自动认领百度教育任务的 Rust 库。
//!
//! ## 功能模块
//!
//! - `api`: 包含所有API响应的数据结构定义
//! - `client`: 包含HTTP客户端和自动认领器
//!
//! ## 基本用法
//!
//! ```rust,no_run
//! use bedu_claim::client::{AutoClaimer, AutoClaimConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = AutoClaimConfig {
//!         server_base_url: "https://easylearn.baidu.com".to_string(),
//!         cookie: "your_cookie_here".to_string(),
//!         task_type: "audittask".to_string(),
//!         claim_limit: 10,
//!         interval: 3.0,
//!         step_id: 1,
//!         subject_id: 2,
//!         clue_type_id: 1,
//!     };
//!
//!     let claimer = AutoClaimer::new(config);
//!     claimer.start().await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## 单独使用HTTP客户端
//!
//! ```rust,no_run
//! use bedu_claim::client::HttpClient;
//! use std::collections::HashMap;
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = HttpClient::new(
//!         "https://easylearn.baidu.com".to_string(),
//!         "your_cookie_here".to_string()
//!     );
//!
//!     // 获取用户信息
//!     let user_info = client.get_user_info().await?;
//!     println!("用户名: {}", user_info.data.user_name);
//!
//!     // 获取任务列表
//!     let mut options = HashMap::new();
//!     options.insert("taskType".to_string(), json!("audittask"));
//!     options.insert("subject".to_string(), json!(2));
//!
//!     let tasks = client.get_audit_task_list(&options).await?;
//!     println!("任务数量: {}", tasks.data.list.len());
//!
//!     Ok(())
//! }
//! ```

pub mod api;
pub mod client;

// 重新导出常用的类型和结构体，方便使用
pub use api::*;
pub use client::{AutoClaimConfig, AutoClaimer, HttpClient};
