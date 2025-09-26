mod api;
mod client;

use anyhow::{Result, anyhow};
use clap::Parser;
use client::{AutoClaimConfig, AutoClaimer};

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

#[tokio::main]
async fn main() -> Result<()> {
    // 使用 env_logger::Builder 来设置默认日志级别
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    // 验证参数
    if args.cookie.is_empty() {
        return Err(anyhow!("Cookie不能为空"));
    }

    if args.interval < 0.001 {
        return Err(anyhow!("轮询间隔不能小于0.001秒（1毫秒）"));
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
