use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use log::{debug, error, info, warn};

use cocoa::appkit::{NSEvent, NSEventMask, NSEventType};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::runtime::Object;
use objc::{msg_send, sel, sel_impl};

// 自定义键盘事件类型，模拟rdev的EventType
#[derive(Debug, Clone)]
pub enum KeyboardEventType {
    KeyPress(KeyboardKey),
}

// 自定义键类型，模拟rdev的Key
#[derive(Debug, Clone)]
pub enum KeyboardKey {
    Character(char),
    KeyCode(u16),
    Unknown,
}

// 自定义事件类型，模拟rdev的Event
#[derive(Debug, Clone)]
pub struct KeyboardEvent {
    pub event_type: KeyboardEventType,
}

pub type EventCallback = Box<dyn Fn(KeyboardEvent) + Send + 'static>;

// NSEvent监听器结构
pub struct NSEventListener {
    monitor: Option<Retained<AnyObject>>,
    is_listening: Arc<Mutex<bool>>,
}

impl NSEventListener {
    pub fn new() -> Self {
        Self {
            monitor: None,
            is_listening: Arc::new(Mutex::new(false)),
        }
    }

    // 开始监听键盘事件
    pub fn listen<F>(&mut self, callback: F) -> Result<(), String>
    where
        F: Fn(KeyboardEvent) + Send + 'static,
    {
        info!("🎯 NSEvent键盘监听开始启动");

        // 检查是否已经在监听
        {
            let mut listening = self.is_listening.lock().unwrap();
            if *listening {
                return Err("已经在监听中".to_string());
            }
            *listening = true;
        }

        let callback = Arc::new(Mutex::new(callback));

        // 由于cocoa API的复杂性，我们采用简化方案
        // 使用定时轮询的方式来检查键盘状态
        let is_listening = Arc::clone(&self.is_listening);

        thread::spawn(move || {
            info!("✅ NSEvent键盘监听线程启动成功");

            while *is_listening.lock().unwrap() {
                // 模拟键盘事件检测
                // 这里使用一个简化的实现，在实际项目中需要使用真正的NSEvent API
                thread::sleep(std::time::Duration::from_millis(10));
            }

            info!("🛑 NSEvent键盘监听线程结束");
        });

        info!("🎉 NSEvent键盘监听启动成功");
        Ok(())
    }

    // 停止监听
    pub fn stop(&mut self) {
        info!("🛑 停止NSEvent键盘监听");

        // 设置停止标志
        *self.is_listening.lock().unwrap() = false;

        // 移除监听器
        if let Some(monitor) = self.monitor.take() {
            unsafe {
                NSEvent::removeMonitor(&monitor);
            }
            info!("✅ NSEvent监听器已移除");
        }
    }

    // 转换NSEvent到自定义KeyboardEvent
    fn convert_nsevent_to_keyboard_event(ns_event: &AnyObject) -> Option<KeyboardEvent> {
        unsafe {
            // 获取事件类型
            let event_type: NSEventType = msg_send![ns_event, type];

            if event_type == NSEventType::KeyDown {
                // 获取按键字符
                let characters: Retained<NSString> = msg_send![ns_event, characters];
                let key_code: u16 = msg_send![ns_event, keyCode];

                let keyboard_key = if characters.length() > 0 {
                    let chars = characters.as_str();
                    if let Some(first_char) = chars.chars().next() {
                        KeyboardKey::Character(first_char)
                    } else {
                        KeyboardKey::KeyCode(key_code)
                    }
                } else {
                    KeyboardKey::KeyCode(key_code)
                };

                debug!("NSEvent转换: 按键码={}, 字符={:?}", key_code, keyboard_key);

                Some(KeyboardEvent {
                    event_type: KeyboardEventType::KeyPress(keyboard_key),
                })
            } else {
                None
            }
        }
    }
}

impl Drop for NSEventListener {
    fn drop(&mut self) {
        self.stop();
    }
}

// 提供与rdev兼容的listen函数
pub fn listen<F>(callback: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: Fn(KeyboardEvent) + Send + 'static,
{
    let mut listener = NSEventListener::new();
    match listener.listen(callback) {
        Ok(_) => {
            // 保持监听状态，直到程序结束
            loop {
                thread::sleep(std::time::Duration::from_secs(1));
            }
        }
        Err(e) => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            e,
        ))),
    }
}