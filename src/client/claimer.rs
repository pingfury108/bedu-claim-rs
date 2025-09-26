use anyhow::{Result, anyhow};
use log::{error, info, warn};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::{interval, sleep};

use crate::api::TaskItem;
use crate::client::HttpClient;

/// 自动认领配置
#[derive(Clone)]
pub struct AutoClaimConfig {
    pub server_base_url: String,
    pub cookie: String,
    pub task_type: String,
    pub claim_limit: i32,
    pub interval: f64,
    pub step_id: i32,
    pub subject_id: i32,
    pub clue_type_id: i32,
}

/// 自动认领器
pub struct AutoClaimer {
    config: AutoClaimConfig,
    client: Arc<HttpClient>,
    successful_claims: Arc<Mutex<i32>>,
    attempt_count: Arc<Mutex<i32>>,
}

impl AutoClaimer {
    /// 创建新的自动认领器实例
    pub fn new(config: AutoClaimConfig) -> Self {
        let client = Arc::new(HttpClient::new(
            config.server_base_url.clone(),
            config.cookie.clone(),
        ));

        Self {
            config,
            client,
            successful_claims: Arc::new(Mutex::new(0)),
            attempt_count: Arc::new(Mutex::new(0)),
        }
    }

    /// 获取当前成功认领的数量
    #[allow(dead_code)]
    pub async fn get_successful_claims(&self) -> i32 {
        *self.successful_claims.lock().await
    }

    /// 获取尝试次数
    #[allow(dead_code)]
    pub async fn get_attempt_count(&self) -> i32 {
        *self.attempt_count.lock().await
    }

    /// 验证Cookie和用户信息
    pub async fn validate_user(&self) -> Result<String> {
        match self.client.get_user_info().await {
            Ok(user_info) => {
                if user_info.errno == 0 {
                    Ok(user_info.data.user_name)
                } else {
                    Err(anyhow!("用户验证失败: {}", user_info.errmsg))
                }
            }
            Err(e) => Err(anyhow!("Cookie验证失败: {}", e)),
        }
    }

    /// 执行单次认领尝试
    pub async fn perform_single_claim(&self) -> Result<i32> {
        let mut attempt_count = self.attempt_count.lock().await;
        *attempt_count += 1;
        let current_attempt = *attempt_count;
        drop(attempt_count);

        let successful_claims = *self.successful_claims.lock().await;

        info!(
            "认领尝试 #{} 开始，当前认领数：{}/{}",
            current_attempt, successful_claims, self.config.claim_limit
        );

        // 检查是否达到认领限制
        if successful_claims >= self.config.claim_limit {
            info!(
                "认领限制已达到 ({}/{})",
                successful_claims, self.config.claim_limit
            );
            return Ok(0);
        }

        // 计算还需要认领多少个任务
        let remaining_claims_needed = self.config.claim_limit - successful_claims;

        // 获取任务列表的选项
        let mut options = HashMap::new();
        options.insert("pn".to_string(), json!(1));
        options.insert("rn".to_string(), json!(20));
        options.insert("clueID".to_string(), json!(""));
        options.insert("clueType".to_string(), json!(self.config.clue_type_id));
        options.insert("step".to_string(), json!(self.config.step_id));
        options.insert("subject".to_string(), json!(self.config.subject_id));
        options.insert("taskType".to_string(), json!(self.config.task_type));

        // 获取任务列表
        let task_response = self.client.get_audit_task_list(&options).await?;

        if task_response.errno != 0 {
            return Err(anyhow!("获取任务列表失败: {}", task_response.errmsg));
        }

        let tasks = task_response.data.list;
        info!("获取到 {} 个任务", tasks.len());

        if tasks.is_empty() {
            warn!("线索池中没任务");
            return Ok(0);
        }

        // 简单筛选
        let filtered_tasks: Vec<TaskItem> = tasks
            .into_iter()
            .take(remaining_claims_needed as usize)
            .collect();

        if filtered_tasks.is_empty() {
            warn!("没有符合条件的任务");
            return Ok(0);
        }

        // 提取任务ID
        let task_ids: Vec<String> = filtered_tasks
            .iter()
            .map(|task| {
                if self.config.task_type == "producetask" {
                    task.clue_id.to_string()
                } else {
                    task.task_id.to_string()
                }
            })
            .collect();

        info!("尝试认领 {} 个任务: {:?}", task_ids.len(), task_ids);

        // 执行认领
        let claim_result = self.claim_tasks(task_ids).await?;

        Ok(claim_result)
    }

    /// 执行认领任务操作
    pub async fn claim_tasks(&self, task_ids: Vec<String>) -> Result<i32> {
        let claim_response = self
            .client
            .claim_audit_task(task_ids.clone(), &self.config.task_type)
            .await?;

        let success_count = if claim_response.errno == 0 {
            // 尝试从响应中提取成功数量
            let count = if let Some(data) = &claim_response.data {
                if let Some(data_obj) = data.as_object() {
                    if let Some(success) = data_obj.get("success").and_then(|v| v.as_i64()) {
                        success as i32
                    } else {
                        task_ids.len() as i32 // 假设全部成功
                    }
                } else {
                    task_ids.len() as i32 // 假设全部成功
                }
            } else {
                task_ids.len() as i32 // 假设全部成功
            };

            let mut successful_claims = self.successful_claims.lock().await;
            *successful_claims += count;

            info!(
                "认领成功：{} 个任务，TaskID: {:?}，总计：{}/{}",
                count, task_ids, *successful_claims, self.config.claim_limit
            );

            count
        } else {
            // 详细记录认领失败信息
            let task_type = if self.config.task_type == "producetask" {
                "ClueID"
            } else {
                "TaskID"
            };

            let data_info = match &claim_response.data {
                Some(data) => format!("响应数据: {}", data),
                None => "响应数据: null".to_string(),
            };

            warn!(
                "认领失败 {}: {:?}，错误码: {}，错误信息: {}，{}",
                task_type, task_ids, claim_response.errno, claim_response.errmsg, data_info
            );

            // 对于特定错误码，可以给出更友好的提示
            match claim_response.errno {
                10003 => {
                    warn!("提示：请先完成待审核的任务后再尝试认领新任务");
                }
                _ => {}
            }

            0
        };

        Ok(success_count)
    }

    /// 开始自动认领循环
    pub async fn start(&self) -> Result<()> {
        info!("开始自动认领任务...");
        info!(
            "配置: 任务类型={}, 认领限制={}, 轮询间隔={:.1}秒, 学科ID={}, 学段ID={}, 线索类型ID={}",
            self.config.task_type,
            self.config.claim_limit,
            self.config.interval,
            self.config.subject_id,
            self.config.step_id,
            self.config.clue_type_id
        );

        // 验证cookie有效性
        let user_name = self.validate_user().await?;
        info!("用户验证成功: {}", user_name);

        let mut interval = interval(Duration::from_secs_f64(self.config.interval));

        loop {
            interval.tick().await;

            let successful_claims = *self.successful_claims.lock().await;
            if successful_claims >= self.config.claim_limit {
                info!("已达到认领限制，停止自动认领");
                break;
            }

            if let Err(e) = self.perform_single_claim().await {
                error!("认领过程出错: {}", e);
                sleep(Duration::from_secs(1)).await;
            }
        }

        let final_claims = *self.successful_claims.lock().await;
        let final_attempts = *self.attempt_count.lock().await;
        info!(
            "自动认领完成，最终认领数：{}/{}，总尝试次数：{}",
            final_claims, self.config.claim_limit, final_attempts
        );

        Ok(())
    }
}
