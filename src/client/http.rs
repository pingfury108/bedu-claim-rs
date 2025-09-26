use anyhow::{Result, anyhow};
use log::debug;
use reqwest::Client;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::time::Duration;

use crate::api::{ClaimResponse, TaskListResponse, UserInfoResponse};

/// HTTP客户端，封装了与百度教育API的所有交互
pub struct HttpClient {
    client: Client,
    base_url: String,
    cookie: String,
}

impl HttpClient {
    /// 创建新的HTTP客户端实例
    pub fn new(base_url: String, cookie: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url,
            cookie,
        }
    }

    /// 获取审核任务列表
    pub async fn get_audit_task_list(
        &self,
        options: &HashMap<String, Value>,
    ) -> Result<TaskListResponse> {
        let task_type = options
            .get("taskType")
            .and_then(|v| v.as_str())
            .unwrap_or("audittask");

        let pn = options.get("pn").and_then(|v| v.as_i64()).unwrap_or(1);
        let rn = options.get("rn").and_then(|v| v.as_i64()).unwrap_or(20);
        let clue_id = options.get("clueID").and_then(|v| v.as_str()).unwrap_or("");
        let clue_type = options
            .get("clueType")
            .and_then(|v| v.as_i64())
            .unwrap_or(1);
        let step = options.get("step").and_then(|v| v.as_i64()).unwrap_or(1);
        let subject = options.get("subject").and_then(|v| v.as_i64()).unwrap_or(2);

        let url = format!(
            "{}/edushop/question/{}/list?pn={}&rn={}&clueID={}&clueType={}&step={}&subject={}",
            self.base_url, task_type, pn, rn, clue_id, clue_type, step, subject
        );

        debug!("请求任务列表: {}", url);

        let response = self
            .client
            .get(&url)
            .header("Cookie", &self.cookie)
            .header("Accept", "application/json")
            .send()
            .await?;

        let body = response.text().await?;
        debug!("任务列表响应: {}", body);

        let parsed: TaskListResponse = serde_json::from_str(&body)
            .map_err(|e| anyhow!("解析任务列表响应失败: {}, body: {}", e, body))?;

        Ok(parsed)
    }

    /// 认领审核任务
    pub async fn claim_audit_task(
        &self,
        task_ids: Vec<String>,
        task_type: &str,
    ) -> Result<ClaimResponse> {
        let commit_type = if task_type == "producetask" {
            "producetaskcommit"
        } else {
            "audittaskcommit"
        };

        let url = format!("{}/edushop/question/{}/claim", self.base_url, commit_type);

        let request_body = if task_type == "producetask" {
            let clue_ids: Result<Vec<u64>, _> = task_ids.iter().map(|s| s.parse()).collect();
            json!({ "clueIDs": clue_ids? })
        } else {
            let task_ids_parsed: Result<Vec<u64>, _> = task_ids.iter().map(|s| s.parse()).collect();
            json!({ "taskIDs": task_ids_parsed? })
        };

        debug!("认领请求: {} -> {}", url, request_body);

        let response = self
            .client
            .post(&url)
            .header("Cookie", &self.cookie)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let body = response.text().await?;
        debug!("认领响应: {}", body);

        let parsed: ClaimResponse = serde_json::from_str(&body)
            .map_err(|e| anyhow!("解析认领响应失败: {}, body: {}", e, body))?;

        Ok(parsed)
    }

    /// 获取用户信息
    pub async fn get_user_info(&self) -> Result<UserInfoResponse> {
        let url = format!("{}/edushop/user/common/info", self.base_url);

        let response = self
            .client
            .get(&url)
            .header("Cookie", &self.cookie)
            .header("Accept", "application/json")
            .send()
            .await?;

        let body = response.text().await?;
        let parsed: UserInfoResponse = serde_json::from_str(&body)?;

        Ok(parsed)
    }
}
