use rodio::{Decoder, OutputStream, Sink};
use serde::{Deserialize, Serialize};

// 引入我们的键盘适配器
mod keyboard_adapter;
use keyboard_adapter::{listen, EventType};


use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIconBuilder, TrayIconEvent,
};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::application::ApplicationHandler;
use chrono::Local;
use log::{debug, error, info, warn};
use simplelog::*;

#[derive(Serialize, Deserialize, Debug)]
struct Settings {
    sound_enabled: bool,
    volume: f32, // 音量范围 0.0 - 1.0
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            sound_enabled: true,
            volume: 0.7, // 默认音量70%
        }
    }
}

struct AppState {
    settings: Arc<Mutex<Settings>>,
    sound_path: Option<PathBuf>,
}

fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
    // 创建日志目录
    let log_dir = dirs::home_dir()
        .ok_or("无法获取用户主目录")?
        .join("Library/Logs/macos-key-sound");
    
    std::fs::create_dir_all(&log_dir)?;
    
    let log_file = log_dir.join(format!("app-{}.log", 
        Local::now().format("%Y%m%d_%H%M%S")));
    
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Debug,
            Config::default(),
            File::create(log_file)?,
        ),
    ])?;
    
    info!("日志系统初始化成功");
    Ok(())
}

impl AppState {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let loaded_settings = load_settings();
        info!("加载的设置: sound_enabled = {}, volume = {:.0}%",
              loaded_settings.sound_enabled, loaded_settings.volume * 100.0);
        let settings = Arc::new(Mutex::new(loaded_settings));
        let sound_path = locate_sound_file();
        if let Some(p) = &sound_path {
            info!("音频文件定位成功: {}", p.display());
        } else {
            warn!("未找到音频文件，请检查安装包内 Resources/assets/sound.wav 是否存在");
        }
        Ok(AppState { settings, sound_path })
    }
    
    fn is_sound_enabled(&self) -> bool {
        self.settings.lock().unwrap().sound_enabled
    }
    
    fn toggle_sound(&self) -> bool {
        let mut settings = self.settings.lock().unwrap();
        settings.sound_enabled = !settings.sound_enabled;
        let enabled = settings.sound_enabled;
        save_settings(&settings);
        info!("音效状态切换: {}", if enabled { "开启" } else { "关闭" });
        enabled
    }

    fn get_volume(&self) -> f32 {
        self.settings.lock().unwrap().volume
    }

    fn set_volume(&self, volume: f32) {
        let mut settings = self.settings.lock().unwrap();
        settings.volume = volume.clamp(0.0, 1.0);
        save_settings(&settings);
        info!("音量设置为: {:.0}%", settings.volume * 100.0);
    }

    fn increase_volume(&self) -> f32 {
        let mut settings = self.settings.lock().unwrap();
        settings.volume = (settings.volume + 0.1).clamp(0.0, 1.0);
        let new_volume = settings.volume;
        save_settings(&settings);
        info!("音量增加到: {:.0}%", new_volume * 100.0);
        new_volume
    }

    fn decrease_volume(&self) -> f32 {
        let mut settings = self.settings.lock().unwrap();
        settings.volume = (settings.volume - 0.1).clamp(0.0, 1.0);
        let new_volume = settings.volume;
        save_settings(&settings);
        info!("音量减少到: {:.0}%", new_volume * 100.0);
        new_volume
    }
    
    fn play_sound(&self) {
        if !self.is_sound_enabled() {
            debug!("音效已关闭，跳过播放");
            return;
        }
        if self.sound_path.is_none() {
            warn!("未配置音频文件路径，取消播放");
            return;
        }
        let sound_path = self.sound_path.clone();
        let volume = self.get_volume();
        debug!("准备播放音效: {:?}, 音量: {:.0}%", sound_path, volume * 100.0);
        thread::spawn(move || {
            if let Some(path) = sound_path {
                debug!("音频线程启动，文件: {}", path.display());
                match OutputStream::try_default() {
                    Ok((_stream, stream_handle)) => {
                        match Sink::try_new(&stream_handle) {
                            Ok(sink) => {
                                // 设置音量
                                sink.set_volume(volume);
                                match File::open(&path) {
                                    Ok(file) => {
                                        let source = BufReader::new(file);
                                        match Decoder::new(source) {
                                            Ok(decoder) => {
                                                sink.append(decoder);
                                                sink.sleep_until_end();
                                                debug!("音效播放完成，音量: {:.0}%", volume * 100.0);
                                            }
                                            Err(e) => error!("音频解码失败: {:?}", e),
                                        }
                                    }
                                    Err(e) => error!("无法打开音频文件 {}: {:?}", path.display(), e),
                                }
                            }
                            Err(e) => error!("创建Sink失败: {:?}", e),
                        }
                    }
                    Err(e) => error!("创建音频输出流失败: {:?}", e),
                }
            }
        });
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    if let Err(e) = init_logging() {
        eprintln!("无法初始化日志系统: {}", e);
    }
    
    info!("MacOS Key Sound GUI - 启动中...");
    
    let event_loop = EventLoop::new()?;
    let app_state = Arc::new(AppState::new()?);
    
    // 创建托盘菜单
    let menu = Menu::new();

    let toggle_item = MenuItem::new(
        if app_state.is_sound_enabled() { "✓ 启用音效" } else { "启用音效" },
        true,
        None
    );

    // 音量控制菜单项 - 平铺显示而不是子菜单
    let volume_up_item = MenuItem::new("🔊 音量+", true, None);
    let volume_down_item = MenuItem::new("🔉 音量-", true, None);
    let current_volume = format!("🎵 当前音量: {:.0}%", app_state.get_volume() * 100.0);
    let volume_display_item = MenuItem::new(&current_volume, false, None);

    // 快捷音量设置
    let volume_25_item = MenuItem::new("🔹 设置为 25%", true, None);
    let volume_50_item = MenuItem::new("🔹 设置为 50%", true, None);
    let volume_75_item = MenuItem::new("🔹 设置为 75%", true, None);
    let volume_100_item = MenuItem::new("🔹 设置为 100%", true, None);

    let separator = PredefinedMenuItem::separator();
    let quit_item = MenuItem::new("退出", true, None);

    menu.append(&toggle_item)?;
    menu.append(&separator)?;
    menu.append(&volume_display_item)?;
    menu.append(&volume_up_item)?;
    menu.append(&volume_down_item)?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&volume_25_item)?;
    menu.append(&volume_50_item)?;
    menu.append(&volume_75_item)?;
    menu.append(&volume_100_item)?;
    menu.append(&separator)?;
    menu.append(&quit_item)?;
    
    // 创建托盘图标
    let icon = create_tray_icon();
    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("MacOS Key Sound - 键盘音效")
        .with_icon(icon)
        .build()?;
    
    // 在后台线程启动键盘监听 - 监听并播放声音
    let app_state_for_keyboard = Arc::clone(&app_state);
    thread::spawn(move || {
        info!("🎯 键盘监听线程已启动 - 监听并播放音效");

        let listen_result = listen(move |event| {
            if let EventType::KeyPress(key) = &event.event_type {
                info!("按下按键: {:?}", key);
                // 播放音效
                app_state_for_keyboard.play_sound();
            }
        });

        match listen_result {
            Ok(_) => {
                info!("✅ 键盘监听正常结束");
            }
            Err(error) => {
                error!("❌ 键盘监听错误: {:?}", error);
                error!("⚠️  请检查辅助功能权限！");
                error!("🔧 解决方案：系统偏好设置 → 安全性与隐私 → 隐私 → 辅助功能");
            }
        }

        info!("🏁 键盘监听线程结束");
    });
    
    info!("应用已启动，请查看系统托盘图标");

    // 主事件循环
    let mut app_handler = TrayApp {
        app_state,
        menu_channel: MenuEvent::receiver().clone(),
        tray_channel: TrayIconEvent::receiver().clone(),
        toggle_item,
        quit_item,
        volume_up_item,
        volume_down_item,
        volume_display_item,
        volume_25_item,
        volume_50_item,
        volume_75_item,
        volume_100_item,
    };
    
    event_loop.run_app(&mut app_handler)?;
    
    Ok(())
}

struct TrayApp {
    app_state: Arc<AppState>,
    menu_channel: crossbeam_channel::Receiver<MenuEvent>,
    tray_channel: crossbeam_channel::Receiver<TrayIconEvent>,
    toggle_item: MenuItem,
    quit_item: MenuItem,
    volume_up_item: MenuItem,
    volume_down_item: MenuItem,
    volume_display_item: MenuItem,
    volume_25_item: MenuItem,
    volume_50_item: MenuItem,
    volume_75_item: MenuItem,
    volume_100_item: MenuItem,
}

impl ApplicationHandler for TrayApp {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        debug!("应用已恢复");
    }

    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        _event: winit::event::WindowEvent,
    ) {
    }

    fn new_events(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _cause: winit::event::StartCause,
    ) {
        event_loop.set_control_flow(ControlFlow::Wait);

        // 处理托盘图标事件
        if let Ok(event) = self.tray_channel.try_recv() {
            debug!("托盘事件: {:?}", event);
        }

        // 处理菜单事件
        if let Ok(event) = self.menu_channel.try_recv() {
            if event.id == self.toggle_item.id() {
                let enabled = self.app_state.toggle_sound();
                self.toggle_item.set_text(if enabled { "✓ 启用音效" } else { "启用音效" });
            } else if event.id == self.volume_up_item.id() {
                let new_volume = self.app_state.increase_volume();
                self.update_volume_display(new_volume);
            } else if event.id == self.volume_down_item.id() {
                let new_volume = self.app_state.decrease_volume();
                self.update_volume_display(new_volume);
            } else if event.id == self.volume_25_item.id() {
                self.app_state.set_volume(0.25);
                self.update_volume_display(0.25);
            } else if event.id == self.volume_50_item.id() {
                self.app_state.set_volume(0.50);
                self.update_volume_display(0.50);
            } else if event.id == self.volume_75_item.id() {
                self.app_state.set_volume(0.75);
                self.update_volume_display(0.75);
            } else if event.id == self.volume_100_item.id() {
                self.app_state.set_volume(1.0);
                self.update_volume_display(1.0);
            } else if event.id == self.quit_item.id() {
                info!("用户请求退出应用");
                std::process::exit(0);
            }
        }
    }
}

impl TrayApp {
    fn update_volume_display(&self, volume: f32) {
        let volume_text = format!("🎵 当前音量: {:.0}%", volume * 100.0);
        self.volume_display_item.set_text(&volume_text);
    }
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
        info!("查找配置文件: {}", config_path.display());
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            info!("配置文件内容: {}", content);
            if let Ok(settings) = serde_json::from_str(&content) {
                info!("成功加载配置文件");
                return settings;
            } else {
                warn!("配置文件解析失败");
            }
        } else {
            info!("配置文件不存在，使用默认设置");
        }
    } else {
        warn!("无法获取配置目录");
    }
    let default_settings = Settings::default();
    info!("使用默认设置: sound_enabled = {}", default_settings.sound_enabled);
    default_settings
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

fn locate_sound_file() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    
    // 1. 开发环境：工作目录中的 assets/sound.wav
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("assets/sound.wav"));
    } else {
        candidates.push(PathBuf::from("assets/sound.wav"));
    }
    
    // 2. macOS 应用包中的资源路径
    if let Ok(exe) = std::env::current_exe() {
        debug!("可执行文件路径: {}", exe.display());
        
        // 方案A: Contents/Resources/assets/sound.wav (标准 macOS 应用包结构)
        if let Some(resources) = exe.parent() // MacOS 目录
            .and_then(|p| p.parent()) // Contents 目录
            .map(|c| c.join("Resources").join("assets").join("sound.wav")) {
            candidates.push(resources.clone());
            debug!("候选路径A: {}", resources.display());
        }
        
        // 方案B: Contents/Resources/sound.wav (直接放在Resources下)
        if let Some(resources) = exe.parent() // MacOS 目录
            .and_then(|p| p.parent()) // Contents 目录
            .map(|c| c.join("Resources").join("sound.wav")) {
            candidates.push(resources.clone());
            debug!("候选路径B: {}", resources.display());
        }
        
        // 方案C: 与可执行文件同目录
        if let Some(exe_dir) = exe.parent() {
            let same_dir = exe_dir.join("sound.wav");
            candidates.push(same_dir.clone());
            debug!("候选路径C: {}", same_dir.display());
            
            let assets_in_exe_dir = exe_dir.join("assets").join("sound.wav");
            candidates.push(assets_in_exe_dir.clone());
            debug!("候选路径D: {}", assets_in_exe_dir.display());
        }
    }
    
    debug!("正在检查 {} 个候选路径...", candidates.len());
    for (i, path) in candidates.iter().enumerate() {
        debug!("检查路径 {}: {} - {}", i+1, path.display(), 
                if path.exists() { "存在" } else { "不存在" });
        if path.exists() {
            info!("✅ 找到音效文件: {}", path.display());
            return Some(path.clone());
        }
    }
    
    error!("❌ 未找到任何音效文件");
    None
}
