use rodio::{Decoder, OutputStream, Sink};
use rdev::{listen, EventType};
use serde::{Deserialize, Serialize};
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
        let settings = Arc::new(Mutex::new(load_settings()));
        let sound_path = locate_sound_file();
        if let Some(p) = &sound_path {
            info!("音频文件定位成功: {}", p.display());
        } else {
            warn!("未找到音频文件，请检查安装包内 Resources/assets/sound.mp3 是否存在");
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
        debug!("准备播放音效: {:?}", sound_path);
        thread::spawn(move || {
            if let Some(path) = sound_path {
                debug!("音频线程启动，文件: {}", path.display());
                match OutputStream::try_default() {
                    Ok((_stream, stream_handle)) => {
                        match Sink::try_new(&stream_handle) {
                            Ok(sink) => {
                                match File::open(&path) {
                                    Ok(file) => {
                                        let source = BufReader::new(file);
                                        match Decoder::new(source) {
                                            Ok(decoder) => {
                                                sink.append(decoder);
                                                sink.sleep_until_end();
                                                debug!("音效播放完成");
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
    
    // 创建通信通道
    let (tx, rx) = std::sync::mpsc::channel();
    
    // 🔧 修改键盘监听 - 使用不同的线程策略避免HIToolbox问题
    let app_state_for_keyboard = Arc::clone(&app_state);
    let tx_clone = tx.clone();
    let _tx_for_error = tx.clone();

    // 尝试使用较短的事件处理来避免长时间在后台线程
    thread::spawn(move || {
        info!("🎯 键盘监听线程已启动");

        // 使用最小化的事件处理避免HIToolbox线程问题
        let listen_result = listen(move |event| {
            // 检查键盘按下事件并打印具体按键
            if let EventType::KeyPress(key) = &event.event_type {
                // 打印按下的具体键
                info!("按下按键: {:?}", key);

                // 立即触发音效
                app_state_for_keyboard.play_sound();
                let _ = tx_clone.send(true);
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
                let _ = _tx_for_error.send(false);
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
        keyboard_status_rx: rx,
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
    keyboard_status_rx: std::sync::mpsc::Receiver<bool>,
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
        
        // 检查键盘监听状态
        if let Ok(status) = self.keyboard_status_rx.try_recv() {
            if status {
                debug!("键盘监听正常工作");
            } else {
                warn!("键盘监听失败，应用功能受限");
            }
        }
        
        // 处理托盘图标事件
        if let Ok(event) = self.tray_channel.try_recv() {
            debug!("托盘事件: {:?}", event);
        }
        
        // 处理菜单事件
        if let Ok(event) = self.menu_channel.try_recv() {
            if event.id == self.toggle_item.id() {
                let enabled = self.app_state.toggle_sound();
                self.toggle_item.set_text(if enabled { "✓ 启用音效" } else { "启用音效" });
            } else if event.id == self.quit_item.id() {
                info!("用户请求退出应用");
                std::process::exit(0);
            }
        }
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

fn locate_sound_file() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    
    // 1. 开发环境：工作目录中的 assets/sound.mp3
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("assets/sound.mp3"));
    } else {
        candidates.push(PathBuf::from("assets/sound.mp3"));
    }
    
    // 2. macOS 应用包中的资源路径
    if let Ok(exe) = std::env::current_exe() {
        debug!("可执行文件路径: {}", exe.display());
        
        // 方案A: Contents/Resources/assets/sound.mp3 (标准 macOS 应用包结构)
        if let Some(resources) = exe.parent() // MacOS 目录
            .and_then(|p| p.parent()) // Contents 目录
            .map(|c| c.join("Resources").join("assets").join("sound.mp3")) {
            candidates.push(resources.clone());
            debug!("候选路径A: {}", resources.display());
        }
        
        // 方案B: Contents/Resources/sound.mp3 (直接放在Resources下)
        if let Some(resources) = exe.parent() // MacOS 目录
            .and_then(|p| p.parent()) // Contents 目录
            .map(|c| c.join("Resources").join("sound.mp3")) {
            candidates.push(resources.clone());
            debug!("候选路径B: {}", resources.display());
        }
        
        // 方案C: 与可执行文件同目录
        if let Some(exe_dir) = exe.parent() {
            let same_dir = exe_dir.join("sound.mp3");
            candidates.push(same_dir.clone());
            debug!("候选路径C: {}", same_dir.display());
            
            let assets_in_exe_dir = exe_dir.join("assets").join("sound.mp3");
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
