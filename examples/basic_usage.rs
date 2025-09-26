use anyhow::Result;
use bedu_claim::client::{AutoClaimConfig, AutoClaimer, HttpClient};
use serde_json::json;
use std::collections::HashMap;

/// 示例1：使用自动认领器，并监控状态
async fn example_auto_claimer() -> Result<()> {
    // 配置自动认领参数
    let config = AutoClaimConfig {
        server_base_url: "https://easylearn.baidu.com".to_string(),
        cookie: "your_cookie_here".to_string(),
        task_type: "audittask".to_string(),
        claim_limit: 5,
        interval: 2.0,
        step_id: 1,
        subject_id: 2,
        clue_type_id: 1,
    };

    // 创建自动认领器
    let claimer = AutoClaimer::new(config);

    // 验证用户
    let user_name = claimer.validate_user().await?;
    println!("用户验证成功: {}", user_name);

    // 执行几次认领并监控状态
    for i in 1..=3 {
        println!("\n--- 第 {} 轮认领 ---", i);

        // 显示当前状态
        let attempts_before = claimer.get_attempt_count().await;
        let claims_before = claimer.get_successful_claims().await;
        println!(
            "认领前状态 - 尝试次数: {}, 成功认领: {}",
            attempts_before, claims_before
        );

        // 执行单次认领
        let claimed = claimer.perform_single_claim().await?;
        println!("本次认领了 {} 个任务", claimed);

        // 显示更新后的状态
        let attempts_after = claimer.get_attempt_count().await;
        let claims_after = claimer.get_successful_claims().await;
        println!(
            "认领后状态 - 尝试次数: {}, 成功认领: {}",
            attempts_after, claims_after
        );

        if claimed == 0 {
            println!("没有更多任务可认领");
            break;
        }
    }

    Ok(())
}

/// 示例2：直接使用HTTP客户端
async fn example_http_client() -> Result<()> {
    let client = HttpClient::new(
        "https://easylearn.baidu.com".to_string(),
        "your_cookie_here".to_string(),
    );

    // 获取用户信息
    let user_info = client.get_user_info().await?;
    if user_info.errno == 0 {
        println!("用户名: {}", user_info.data.user_name);
        println!("角色: {:?}", user_info.data.role_names);
    } else {
        println!("获取用户信息失败: {}", user_info.errmsg);
    }

    // 获取任务列表
    let mut options = HashMap::new();
    options.insert("taskType".to_string(), json!("audittask"));
    options.insert("subject".to_string(), json!(2));
    options.insert("step".to_string(), json!(1));
    options.insert("clueType".to_string(), json!(1));

    let tasks = client.get_audit_task_list(&options).await?;
    if tasks.errno == 0 {
        println!("任务总数: {}", tasks.data.total);
        println!("当前页任务数: {}", tasks.data.list.len());

        for task in &tasks.data.list {
            println!("任务 {}: {}", task.task_id, task.brief);
        }

        // 如果有任务，尝试认领第一个
        if !tasks.data.list.is_empty() {
            let task_ids = vec![tasks.data.list[0].task_id.to_string()];
            let claim_result = client.claim_audit_task(task_ids, "audittask").await?;

            if claim_result.errno == 0 {
                println!("认领成功!");
            } else {
                println!("认领失败: {}", claim_result.errmsg);
            }
        }
    } else {
        println!("获取任务列表失败: {}", tasks.errmsg);
    }

    Ok(())
}

/// 示例4：状态监控器
async fn example_status_monitor() -> Result<()> {
    let config = AutoClaimConfig {
        server_base_url: "https://easylearn.baidu.com".to_string(),
        cookie: "your_cookie_here".to_string(),
        task_type: "audittask".to_string(),
        claim_limit: 10,
        interval: 1.0,
        step_id: 1,
        subject_id: 2,
        clue_type_id: 1,
    };

    let claimer = AutoClaimer::new(config);

    // 模拟状态监控
    println!("=== 认领器状态监控 ===");

    // 初始状态
    let initial_attempts = claimer.get_attempt_count().await;
    let initial_claims = claimer.get_successful_claims().await;
    println!(
        "初始状态 - 尝试: {}, 成功: {}",
        initial_attempts, initial_claims
    );

    // 执行多次认领，每次都检查状态
    for round in 1..=5 {
        match claimer.perform_single_claim().await {
            Ok(claimed) => {
                let attempts = claimer.get_attempt_count().await;
                let total_claims = claimer.get_successful_claims().await;

                println!(
                    "第{}轮 - 本次认领: {}, 总尝试: {}, 总成功: {} (成功率: {:.1}%)",
                    round,
                    claimed,
                    attempts,
                    total_claims,
                    if attempts > 0 {
                        (total_claims as f32 / attempts as f32) * 100.0
                    } else {
                        0.0
                    }
                );

                if claimed == 0 {
                    println!("⚠️  没有更多任务，停止监控");
                    break;
                }
            }
            Err(e) => {
                println!("❌ 第{}轮认领失败: {}", round, e);
            }
        }

        // 添加延迟，模拟监控间隔
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    // 最终统计
    let final_attempts = claimer.get_attempt_count().await;
    let final_claims = claimer.get_successful_claims().await;
    println!("\n📊 最终统计:");
    println!("   总尝试次数: {}", final_attempts);
    println!("   成功认领数: {}", final_claims);
    println!(
        "   成功率: {:.1}%",
        if final_attempts > 0 {
            (final_claims as f32 / final_attempts as f32) * 100.0
        } else {
            0.0
        }
    );

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("=== 百度教育API库使用示例 ===\n");

    println!("示例1: 自动认领器状态监控");
    if let Err(e) = example_auto_claimer().await {
        println!("错误: {}", e);
    }

    println!("\n示例2: 直接使用HTTP客户端");
    if let Err(e) = example_http_client().await {
        println!("错误: {}", e);
    }

    println!("\n示例3: 认领器状态监控");
    if let Err(e) = example_status_monitor().await {
        println!("错误: {}", e);
    }

    Ok(())
}
