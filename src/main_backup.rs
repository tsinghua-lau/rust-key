use rodio::{Decoder, OutputStream, Sink};
use rdev::{listen, EventType};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, Mutex};
use std::thread;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder, TrayIconEvent,
};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::application::ApplicationHandler;

#[derive(Serialize, Deserialize, Debug)]
struct Settings {
    sound_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            sound_enabled: true,
        }
    }
}

struct AppState {
    settings: Arc<Mutex<Settings>>,
}

impl AppState {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // 检查音频文件是否存在
        if !std::path::Path::new("assets/sound.wav").exists() {
            eprintln!("警告: 音频文件 assets/sound.wav 不存在");
        }
        
        let settings = Arc::new(Mutex::new(load_settings()));
        
        Ok(AppState {
            settings,
        })
    }
    
    fn is_sound_enabled(&self) -> bool {
        self.settings.lock().unwrap().sound_enabled
    }
    
    fn toggle_sound(&self) -> bool {
        let mut settings = self.settings.lock().unwrap();
        settings.sound_enabled = !settings.sound_enabled;
        let enabled = settings.sound_enabled;
        save_settings(&settings);
        println!("音效状态: {}", if enabled { "开启" } else { "关闭" });
        enabled
    }
    
    fn play_sound(&self) {
        if !self.is_sound_enabled() {
            println!("音效已关闭，不播放");
            return;
        }
        
        println!("尝试播放音效...");
        
        // 在单独的线程中创建音频流，避免线程安全问题
        thread::spawn(move || {
            println!("音频线程启动");
            
            match OutputStream::try_default() {
                Ok((_stream, stream_handle)) => {
                    println!("音频输出流创建成功");
                    
                    match Sink::try_new(&stream_handle) {
                        Ok(sink) => {
                            println!("Sink创建成功");
                            
                            match File::open("assets/sound.wav") {
                                Ok(file) => {
                                    println!("音频文件打开成功");
                                    let source = BufReader::new(file);
                                    
                                    match Decoder::new(source) {
                                        Ok(decoder) => {
                                            println!("音频解码器创建成功，开始播放");
                                            sink.append(decoder);
                                            sink.sleep_until_end();
                                            println!("音频播放完成");
                                        }
                                        Err(e) => {
                                            eprintln!("音频解码失败: {:?}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("无法打开音频文件: {:?}", e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("创建Sink失败: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("创建音频输出流失败: {:?}", e);
                }
            }
        });
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("MacOS Key Sound GUI - 启动中...");
    
    let event_loop = EventLoop::new()?;
    let app_state = Arc::new(AppState::new()?);
    
    // 创建托盘菜单
    let menu = Menu::new();
    
    let toggle_item = MenuItem::new(
        if app_state.is_sound_enabled() { "✓ 启用音效" } else { "启用音效" },
        true,
        None
    );
    let separator = PredefinedMenuItem::separator();
    let quit_item = MenuItem::new("退出", true, None);
    
    menu.append(&toggle_item)?;
    menu.append(&separator)?;
    menu.append(&quit_item)?;
    
    // 创建托盘图标
    let icon = create_tray_icon();
    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("MacOS Key Sound - 键盘音效")
        .with_icon(icon)
        .build()?;
    
    // 启动键盘监听线程
    let app_state_for_keyboard = Arc::clone(&app_state);
    thread::spawn(move || {
        println!("开始监听键盘事件...");
        if let Err(error) = listen(move |event| {
            match event.event_type {
                EventType::KeyPress(_) => {
                    app_state_for_keyboard.play_sound();
                }
                _ => {}
            }
        }) {
            eprintln!("键盘监听错误: {:?}", error);
            eprintln!("请在系统偏好设置 > 安全性与隐私 > 隐私 > 辅助功能中授权此应用");
        }
    });
    
    println!("应用已启动，请查看系统托盘图标");
    println!("注意: 首次运行时，macOS 可能会提示授予辅助功能权限");
    
        // 主事件循环
    let mut app_handler = TrayApp {
        app_state,
        menu_channel: MenuEvent::receiver(),
        tray_channel: TrayIconEvent::receiver(),
        toggle_item,
        quit_item,
    };
    
    // 使用新的 run_app 方法
    event_loop.run_app(&mut app_handler)?;
    
    Ok(())
}

struct TrayApp {
    app_state: Arc<AppState>,
    menu_channel: crossbeam_channel::Receiver<MenuEvent>,
    tray_channel: crossbeam_channel::Receiver<TrayIconEvent>,
    toggle_item: MenuItem,
    quit_item: MenuItem,
}

impl ApplicationHandler for TrayApp {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        // 应用恢复时的处理
    }

    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        _event: winit::event::WindowEvent,
    ) {
        // 窗口事件处理（我们是托盘应用，不需要窗口）
    }

    fn new_events(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _cause: winit::event::StartCause,
    ) {
        event_loop.set_control_flow(ControlFlow::Wait);
        
        // 处理托盘图标事件
        if let Ok(event) = self.tray_channel.try_recv() {
            println!("托盘事件: {:?}", event);
        }
        
        // 处理菜单事件
        if let Ok(event) = self.menu_channel.try_recv() {
            if event.id == self.toggle_item.id() {
                let enabled = self.app_state.toggle_sound();
                // 更新菜单项文本
                self.toggle_item.set_text(if enabled { "✓ 启用音效" } else { "启用音效" });
            } else if event.id == self.quit_item.id() {
                println!("退出应用");
                std::process::exit(0);
            }
        }
    }", event);
        }
        
        // 处理菜单事件
        if let Ok(event) = menu_channel.try_recv() {
            if event.id == toggle_item.id() {
                let enabled = app_state.toggle_sound();
                // 更新菜单项文本
                self.toggle_item.set_text(if enabled { "✓ 启用音效" } else { "启用音效" });
            } else if event.id == quit_item.id() {
                println!("退出应用");
                std::process::exit(0);
            }
        }
    })?;
    
    Ok(())
}

fn create_tray_icon() -> tray_icon::Icon {
    // 创建一个简单的16x16像素的音符图标
    let mut rgba = vec![0u8; 16 * 16 * 4]; // 16x16 RGBA
    
    // 绘制一个简单的音符图标
    for y in 0..16 {
        for x in 0..16 {
            let idx = (y * 16 + x) * 4;
            
            // 绘制音符的竖线 (x=8, y=2-13)
            if x == 8 && y >= 2 && y <= 13 {
                rgba[idx] = 255;     // R
                rgba[idx + 1] = 255; // G  
                rgba[idx + 2] = 255; // B
                rgba[idx + 3] = 255; // A
            }
            // 绘制音符的符头 (椭圆形, 底部)
            else if ((x == 6 || x == 7 || x == 9 || x == 10) && (y == 11 || y == 12)) ||
                    ((x == 7 || x == 8 || x == 9) && (y == 13)) {
                rgba[idx] = 255;     // R
                rgba[idx + 1] = 255; // G
                rgba[idx + 2] = 255; // B
                rgba[idx + 3] = 255; // A
            }
            // 绘制音符的符尾 (顶部的弧线)
            else if ((x == 9 || x == 10 || x == 11) && y == 2) ||
                    ((x == 10 || x == 11) && y == 3) ||
                    (x == 11 && (y == 4 || y == 5)) {
                rgba[idx] = 255;     // R
                rgba[idx + 1] = 255; // G
                rgba[idx + 2] = 255; // B
                rgba[idx + 3] = 255; // A
            }
        }
    }
    
    tray_icon::Icon::from_rgba(rgba, 16, 16).expect("创建图标失败")
}

fn load_settings() -> Settings {
    if let Some(config_dir) = dirs::config_dir() {
        let config_path = config_dir.join("macos-key-sound").join("settings.json");
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(settings) = serde_json::from_str(&content) {
                return settings;
            }
        }
    }
    Settings::default()
}

fn save_settings(settings: &Settings) {
    if let Some(config_dir) = dirs::config_dir() {
        let config_dir = config_dir.join("macos-key-sound");
        if std::fs::create_dir_all(&config_dir).is_ok() {
            let config_path = config_dir.join("settings.json");
            if let Ok(content) = serde_json::to_string_pretty(settings) {
                let _ = std::fs::write(config_path, content);
            }
        }
    }
}
