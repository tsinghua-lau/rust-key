// 键盘事件适配层 - 提供与rdev兼容的接口
use log::{debug, error, info, warn};
use std::sync::{Arc, Mutex};
use std::thread;

// 为了保持与rdev的兼容性，重新定义相同的类型
#[derive(Debug, Clone)]
pub enum EventType {
    KeyPress(Key),
}

#[derive(Debug, Clone)]
pub enum Key {
    Alt,
    AltGr,
    Backspace,
    CapsLock,
    ControlLeft,
    ControlRight,
    Delete,
    DownArrow,
    End,
    Escape,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    Home,
    LeftArrow,
    MetaLeft,
    MetaRight,
    PageDown,
    PageUp,
    Return,
    RightArrow,
    ShiftLeft,
    ShiftRight,
    Space,
    Tab,
    UpArrow,
    PrintScreen,
    ScrollLock,
    Pause,
    NumLock,
    BackQuote,
    Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9, Num0,
    Minus,
    Equal,
    KeyQ, KeyW, KeyE, KeyR, KeyT, KeyY, KeyU, KeyI, KeyO, KeyP,
    LeftBracket,
    RightBracket,
    KeyA, KeyS, KeyD, KeyF, KeyG, KeyH, KeyJ, KeyK, KeyL,
    SemiColon,
    Quote,
    BackSlash,
    IntlBackslash,
    KeyZ, KeyX, KeyC, KeyV, KeyB, KeyN, KeyM,
    Comma,
    Dot,
    Slash,
    Insert,
    KpReturn,
    KpMinus,
    KpPlus,
    KpMultiply,
    KpDivide,
    Kp0, Kp1, Kp2, Kp3, Kp4, Kp5, Kp6, Kp7, Kp8, Kp9,
    KpDelete,
    Function,
    Unknown(u32),
}

#[derive(Debug, Clone)]
pub struct Event {
    pub event_type: EventType,
}

// 使用macOS原生API进行键盘监听的结构
pub struct KeyboardMonitor {
    is_listening: Arc<Mutex<bool>>,
}

impl KeyboardMonitor {
    pub fn new() -> Self {
        Self {
            is_listening: Arc::new(Mutex::new(false)),
        }
    }

    // 临时实现：使用NSEvent的简化版本
    pub fn start_monitoring<F>(&mut self, callback: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: Fn(Event) + Send + Sync + 'static,
    {
        info!("🎯 启动键盘监听 (NSEvent适配器)");

        {
            let mut listening = self.is_listening.lock().unwrap();
            if *listening {
                return Err("已经在监听中".into());
            }
            *listening = true;
        }

        let is_listening = Arc::clone(&self.is_listening);
        let callback = Arc::new(callback);

        thread::spawn(move || {
            info!("✅ 键盘监听线程启动 (权限配置版本)");

            // 检查是否有必要的权限
            info!("📝 应用需要以下权限才能监听键盘事件:");
            info!("   1. 辅助功能权限 (Accessibility)");
            info!("   2. 输入监控权限 (Input Monitoring)");
            info!("🔧 请前往: 系统偏好设置 → 安全性与隐私 → 隐私");
            info!("   将此应用添加到 '辅助功能' 和 '输入监控' 列表中");

            // NSEvent监听需要在主线程上运行，这里提供占位实现
            info!("⚠️  当前为权限测试版本，需要在主线程实现真正的NSEvent监听");

            // 保持监听状态但不触发事件
            while *is_listening.lock().unwrap() {
                thread::sleep(std::time::Duration::from_millis(1000));
            }

            info!("🛑 键盘监听线程结束");
        });

        Ok(())
    }

    pub fn stop(&mut self) {
        info!("🛑 停止键盘监听");
        *self.is_listening.lock().unwrap() = false;
    }
}

// 提供与rdev兼容的listen函数
pub fn listen<F>(callback: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: Fn(Event) + Send + Sync + 'static,
{
    let mut monitor = KeyboardMonitor::new();
    monitor.start_monitoring(callback)?;

    // 保持监听状态
    loop {
        thread::sleep(std::time::Duration::from_secs(1));
        if !*monitor.is_listening.lock().unwrap() {
            break;
        }
    }

    Ok(())
}

// 将macOS keyCode转换为我们的Key枚举
fn keycode_to_key(keycode: u16) -> Key {
    match keycode {
        36 => Key::Return,
        53 => Key::Escape,
        48 => Key::Tab,
        49 => Key::Space,
        51 => Key::Backspace,
        117 => Key::Delete,
        123 => Key::LeftArrow,
        124 => Key::RightArrow,
        125 => Key::DownArrow,
        126 => Key::UpArrow,
        18 => Key::Num1,
        19 => Key::Num2,
        20 => Key::Num3,
        21 => Key::Num4,
        23 => Key::Num5,
        22 => Key::Num6,
        26 => Key::Num7,
        28 => Key::Num8,
        25 => Key::Num9,
        29 => Key::Num0,
        12 => Key::KeyQ,
        13 => Key::KeyW,
        14 => Key::KeyE,
        15 => Key::KeyR,
        17 => Key::KeyT,
        16 => Key::KeyY,
        32 => Key::KeyU,
        34 => Key::KeyI,
        31 => Key::KeyO,
        35 => Key::KeyP,
        0 => Key::KeyA,
        1 => Key::KeyS,
        2 => Key::KeyD,
        3 => Key::KeyF,
        5 => Key::KeyG,
        4 => Key::KeyH,
        38 => Key::KeyJ,
        40 => Key::KeyK,
        37 => Key::KeyL,
        6 => Key::KeyZ,
        7 => Key::KeyX,
        8 => Key::KeyC,
        9 => Key::KeyV,
        11 => Key::KeyB,
        45 => Key::KeyN,
        46 => Key::KeyM,
        _ => Key::Unknown(keycode as u32),
    }
}