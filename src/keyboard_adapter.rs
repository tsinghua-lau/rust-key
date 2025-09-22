// 键盘事件适配层 - 使用CGEventTap实现键盘监听
use log::{error, info};
use core_graphics::event::{CGEvent, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType, CGEventTapProxy, CGEventField};
use core_foundation::runloop::{CFRunLoop, kCFRunLoopCommonModes, CFRunLoopRun};

// 键盘事件类型定义
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

// 全局回调函数存储
static mut GLOBAL_CALLBACK: Option<Box<dyn Fn(Event) + Send + Sync>> = None;

// CGEventTap回调函数
fn event_tap_callback(
    _proxy: CGEventTapProxy,
    event_type: CGEventType,
    event: &CGEvent,
) -> Option<CGEvent> {
    // 只处理键盘按下事件
    match event_type {
        CGEventType::KeyDown => {
            let keycode = event.get_integer_value_field(9);
            let key = keycode_to_key(keycode as u16);

            // 打印键盘事件信息
            println!("键盘按下: {:?} (keycode: {})", key, keycode);
            info!("键盘按下: {:?} (keycode: {})", key, keycode);

            let keyboard_event = Event {
                event_type: EventType::KeyPress(key),
            };

            // 调用全局回调函数
            unsafe {
                if let Some(ref callback) = GLOBAL_CALLBACK {
                    callback(keyboard_event);
                }
            }
        }
        _ => {}
    }

    // 返回None表示不拦截事件，让它继续传递
    None
}

// 提供与rdev兼容的listen函数，使用CGEventTap实现真实键盘监听
pub fn listen<F>(callback: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: Fn(Event) + Send + Sync + 'static,
{
    info!("🎯 启动CGEventTap键盘监听");

    // 将回调函数存储到全局变量
    unsafe {
        GLOBAL_CALLBACK = Some(Box::new(callback));
    }

    // 创建要监听的事件类型向量
    let event_types = vec![CGEventType::KeyDown];

    // 创建CGEventTap
    let event_tap = CGEventTap::new(
        CGEventTapLocation::HID,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::ListenOnly,
        event_types,
        event_tap_callback,
    );

    match event_tap {
        Ok(tap) => {
            info!("✅ CGEventTap创建成功");

            // 创建运行循环源
            let run_loop_source = tap.mach_port.create_runloop_source(0);

            match run_loop_source {
                Ok(source) => {
                    info!("✅ 运行循环源创建成功");

                    // 获取当前运行循环
                    let run_loop = CFRunLoop::get_current();

                    // 添加源到运行循环
                    run_loop.add_source(&source, unsafe { kCFRunLoopCommonModes });

                    // 启用事件监听
                    tap.enable();

                    info!("🎧 开始监听键盘事件...");
                    println!("键盘监听已启动，按任意键测试...");

                    // 运行事件循环
                    unsafe { CFRunLoopRun(); }

                    Ok(())
                }
                Err(e) => {
                    error!("❌ 创建运行循环源失败: {:?}", e);
                    Err(format!("无法创建运行循环源: {:?}", e).into())
                }
            }
        }
        Err(e) => {
            error!("❌ CGEventTap创建失败: {:?}", e);
            error!("⚠️  请检查辅助功能权限！");
            error!("🔧 解决方案：系统偏好设置 → 安全性与隐私 → 隐私 → 辅助功能");
            error!("   将此应用添加到辅助功能列表中");
            Err(format!("CGEventTap创建失败: {:?}", e).into())
        }
    }
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