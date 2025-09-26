use anyhow::{Result, anyhow};
use clap::Parser;
use log::{debug, error, info, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::{interval, sleep};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Subject {
    id: i32,
    name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Filter {
    id: String,
    name: String,
    #[serde(rename = "type")]
    filter_type: String,
    list: Vec<Subject>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LabelResponse {
    errno: i32,
    errmsg: String,
    data: LabelData,
}

#[derive(Debug, Serialize, Deserialize)]
struct LabelData {
    filter: Vec<Filter>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TaskItem {
    #[serde(rename = "taskID")]
    task_id: i32,
    #[serde(rename = "clueID")]
    clue_id: i32,
    brief: String,
    step: i32,
    subject: i32,
    state: i32,
    #[serde(rename = "stepName")]
    step_name: String,
    #[serde(rename = "subjectName")]
    subject_name: String,
    #[serde(rename = "clueType")]
    clue_type: i32,
    #[serde(rename = "clueTypeName")]
    clue_type_name: String,
    #[serde(rename = "stateName")]
    state_name: String,
    #[serde(rename = "createTime")]
    create_time: String,
    #[serde(rename = "dispatchTime", default)]
    dispatch_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TaskListData {
    total: i32,
    list: Vec<TaskItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TaskListResponse {
    errno: i32,
    errmsg: String,
    data: TaskListData,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ClaimResponse {
    errno: i32,
    errmsg: String,
    #[serde(default)]
    data: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct UserInfoData {
    #[serde(rename = "roleLinks")]
    role_links: Vec<String>,
    #[serde(rename = "roleNames")]
    role_names: Vec<String>,
    #[serde(rename = "userName")]
    user_name: String,
    avatar: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct UserInfoResponse {
    errno: i32,
    errmsg: String,
    data: UserInfoData,
}

#[derive(Parser, Debug)]
#[command(author, version, about = "百度教育自动认领工具", long_about = None)]
struct Args {
    #[arg(short, long, help = "Cookie字符串")]
    cookie: String,

    #[arg(short, long, default_value = "2", help = "学科ID")]
    subject_id: i32,

    #[arg(short = 'e', long, default_value = "1", help = "学段ID")]
    step_id: i32,

    #[arg(short = 'u', long, default_value = "1", help = "线索类型ID")]
    clue_type_id: i32,

    #[arg(
        short,
        long,
        default_value = "audittask",
        help = "任务类型 (audittask/producetask)"
    )]
    task_type: String,

    #[arg(short = 'l', long, default_value = "10", help = "认领限制数量")]
    limit: i32,

    #[arg(short, long, default_value = "3.0", help = "轮询间隔 (秒)")]
    interval: f64,

    #[arg(
        long,
        default_value = "https://easylearn.baidu.com",
        help = "服务器基础URL"
    )]
    server: String,
}

struct HttpClient {
    client: Client,
    base_url: String,
    cookie: String,
}

impl HttpClient {
    fn new(base_url: String, cookie: String) -> Self {
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

    async fn get_audit_task_list(
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

    async fn claim_audit_task(
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

    async fn get_user_info(&self) -> Result<UserInfoResponse> {
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

#[derive(Clone)]
struct AutoClaimConfig {
    server_base_url: String,
    cookie: String,
    task_type: String,
    claim_limit: i32,
    interval: f64,
    step_id: i32,
    subject_id: i32,
    clue_type_id: i32,
}

struct AutoClaimer {
    config: AutoClaimConfig,
    client: Arc<HttpClient>,
    successful_claims: Arc<Mutex<i32>>,
    attempt_count: Arc<Mutex<i32>>,
}

impl AutoClaimer {
    fn new(config: AutoClaimConfig) -> Self {
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

    async fn perform_auto_claiming(&self) -> Result<()> {
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
            return Ok(());
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
            return Ok(());
        }

        // 简单筛选（这里可以根据需要添加关键词筛选等）
        let filtered_tasks: Vec<TaskItem> = tasks
            .into_iter()
            .take(remaining_claims_needed as usize)
            .collect();

        if filtered_tasks.is_empty() {
            warn!("没有符合条件的任务");
            return Ok(());
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

        // 批量认领任务
        let claim_response = self
            .client
            .claim_audit_task(task_ids.clone(), &self.config.task_type)
            .await?;

        if claim_response.errno == 0 {
            // 尝试从响应中提取成功数量
            let success_count = if let Some(data) = &claim_response.data {
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
            *successful_claims += success_count;

            info!(
                "认领成功：{} 个任务，TaskID: {:?}，总计：{}/{}",
                success_count, task_ids, *successful_claims, self.config.claim_limit
            );

            // 检查是否达到限制
            if *successful_claims >= self.config.claim_limit {
                info!(
                    "认领限制已达到 ({}/{})",
                    *successful_claims, self.config.claim_limit
                );
                return Ok(());
            }
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
        }

        Ok(())
    }

    async fn start(&self) -> Result<()> {
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
        match self.client.get_user_info().await {
            Ok(user_info) => {
                if user_info.errno == 0 {
                    info!("用户验证成功: {}", user_info.data.user_name);
                } else {
                    return Err(anyhow!("用户验证失败: {}", user_info.errmsg));
                }
            }
            Err(e) => {
                return Err(anyhow!("Cookie验证失败: {}", e));
            }
        }

        let mut interval = interval(Duration::from_secs_f64(self.config.interval));

        loop {
            interval.tick().await;

            let successful_claims = *self.successful_claims.lock().await;
            if successful_claims >= self.config.claim_limit {
                info!("已达到认领限制，停止自动认领");
                break;
            }

            if let Err(e) = self.perform_auto_claiming().await {
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

#[tokio::main]
async fn main() -> Result<()> {
    // 使用 env_logger::Builder 来设置默认日志级别
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    // 验证参数
    if args.cookie.is_empty() {
        return Err(anyhow!("Cookie不能为空"));
    }

    if args.interval < 0.1 {
        return Err(anyhow!("轮询间隔不能小于0.1秒"));
    }

    if !["audittask", "producetask"].contains(&args.task_type.as_str()) {
        return Err(anyhow!("任务类型必须是 audittask 或 producetask"));
    }

    let config = AutoClaimConfig {
        server_base_url: args.server,
        cookie: args.cookie,
        task_type: args.task_type,
        claim_limit: args.limit,
        interval: args.interval,
        step_id: args.step_id,
        subject_id: args.subject_id,
        clue_type_id: args.clue_type_id,
    };

    let auto_claimer = AutoClaimer::new(config);
    auto_claimer.start().await?;

    Ok(())
}
