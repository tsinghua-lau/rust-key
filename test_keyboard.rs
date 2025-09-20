use rdev::{listen, EventType};
use log::{debug, error, info, warn};
use simplelog::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化简单日志
    TermLogger::init(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )?;

    info!("🚀 开始键盘监听测试");
    info!("请按任意键进行测试...");

    // 简单的键盘监听测试
    match listen(|event| {
        match event.event_type {
            EventType::KeyPress(key) => {
                info!("✅ 检测到按键: {:?}", key);
            }
            EventType::KeyRelease(key) => {
                debug!("🔄 按键释放: {:?}", key);
            }
            _ => {
                debug!("📋 其他事件: {:?}", event.event_type);
            }
        }
    }) {
        Ok(_) => info!("键盘监听结束"),
        Err(error) => {
            error!("键盘监听失败: {:?}", error);
            error!("可能的原因:");
            error!("1. 辅助功能权限未授予");
            error!("2. rdev 库与当前 macOS 版本不兼容");
            error!("3. 应用需要从命令行运行而不是从应用包运行");
        }
    }

    Ok(())
}