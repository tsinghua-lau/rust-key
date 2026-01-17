use rodio::{Decoder, OutputStream, Sink};
use serde::{Deserialize, Serialize};
use image::{DynamicImage, ImageBuffer, Rgba};

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
use winit::window::Window;
use winit::event::{WindowEvent, ElementState};
use winit::dpi::{LogicalSize, LogicalPosition};
// 暂时移除pixels依赖，使用简化的滑动条实现
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

    // 音量控制菜单项 - 4个固定音量选项
    let current_volume = format!("🎵 当前音量: {:.0}%", app_state.get_volume() * 100.0);
    let volume_display_item = MenuItem::new(&current_volume, false, None);

    let volume_25_item = MenuItem::new("🔉 25%", true, None);
    let volume_50_item = MenuItem::new("🔊 50%", true, None);
    let volume_75_item = MenuItem::new("🔊 75%", true, None);
    let volume_100_item = MenuItem::new("🔊 100%", true, None);

    let separator = PredefinedMenuItem::separator();
    let quit_item = MenuItem::new("退出", true, None);

    menu.append(&toggle_item)?;
    menu.append(&separator)?;
    menu.append(&volume_display_item)?;
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
        app_state: Arc::clone(&app_state),
        menu_channel: MenuEvent::receiver().clone(),
        tray_channel: TrayIconEvent::receiver().clone(),
        toggle_item,
        quit_item,
        volume_display_item,
        volume_25_item,
        volume_50_item,
        volume_75_item,
        volume_100_item,
    };

    // 初始化音量显示标记
    let initial_volume = app_state.get_volume();
    app_handler.update_volume_marks(initial_volume);
    
    event_loop.run_app(&mut app_handler)?;
    
    Ok(())
}

struct TrayApp {
    app_state: Arc<AppState>,
    menu_channel: crossbeam_channel::Receiver<MenuEvent>,
    tray_channel: crossbeam_channel::Receiver<TrayIconEvent>,
    toggle_item: MenuItem,
    quit_item: MenuItem,
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
        // 简化实现，不需要窗口事件处理
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
            } else if event.id == self.volume_25_item.id() {
                self.app_state.set_volume(0.25);
                self.update_volume_display(0.25);
                self.update_volume_marks(0.25);
            } else if event.id == self.volume_50_item.id() {
                self.app_state.set_volume(0.50);
                self.update_volume_display(0.50);
                self.update_volume_marks(0.50);
            } else if event.id == self.volume_75_item.id() {
                self.app_state.set_volume(0.75);
                self.update_volume_display(0.75);
                self.update_volume_marks(0.75);
            } else if event.id == self.volume_100_item.id() {
                self.app_state.set_volume(1.0);
                self.update_volume_display(1.0);
                self.update_volume_marks(1.0);
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

    fn update_volume_marks(&self, current_volume: f32) {
        // 为当前选中的音量级别添加标记
        let current_percent = (current_volume * 100.0).round() as u8;

        // 更新4个音量项的显示，当前音量级别显示为选中状态
        let volumes = [
            (&self.volume_25_item, 25, "🔉 25%"),
            (&self.volume_50_item, 50, "🔊 50%"),
            (&self.volume_75_item, 75, "🔊 75%"),
            (&self.volume_100_item, 100, "🔊 100%"),
        ];

        for (item, level, base_text) in volumes {
            if level == current_percent {
                // 当前选中的音量级别，添加选中标记
                let marked_text = format!("▶ {}", base_text);
                item.set_text(&marked_text);
            } else {
                // 其他级别，显示普通文本
                item.set_text(base_text);
            }
        }
    }
}

fn create_tray_icon() -> tray_icon::Icon {
    // 尝试从文件加载图标，如果失败则使用程序化生成的后备图标
    if let Some(icon) = load_tray_icon_from_file() {
        return icon;
    }

    // 后备方案：程序化生成图标
    create_fallback_tray_icon()
}

fn load_tray_icon_from_file() -> Option<tray_icon::Icon> {
    // 构建多个可能的图标路径
    let mut icon_paths = Vec::new();

    // 1. 开发环境路径
    icon_paths.push("assets/key-icon.png".to_string());
    icon_paths.push("assets/tray-icon.png".to_string());
    icon_paths.push("assets/status-icon.png".to_string());

    // 2. macOS应用包中的路径
    if let Ok(exe) = std::env::current_exe() {
        if let Some(resources) = exe.parent() // MacOS 目录
            .and_then(|p| p.parent()) // Contents 目录
            .map(|c| c.join("Resources")) {

            let app_icon_paths = [
                resources.join("assets").join("key-icon.png"),
                resources.join("assets").join("tray-icon.png"),
                resources.join("assets").join("status-icon.png"),
                resources.join("key-icon.png"), // 直接在Resources下
            ];

            for path in &app_icon_paths {
                icon_paths.push(path.to_string_lossy().to_string());
            }
        }
    }

    for path in &icon_paths {
        if std::path::Path::new(path).exists() {
            info!("🎯 找到状态栏图标文件: {}", path);

            match load_png_as_tray_icon(path) {
                Ok(icon) => {
                    info!("✅ 成功从文件加载状态栏图标: {}", path);
                    return Some(icon);
                }
                Err(e) => {
                    warn!("❌ 加载状态栏图标失败 {}: {}", path, e);
                }
            }
        }
    }

    info!("⚠️  未找到状态栏图标文件，使用程序化生成的图标");
    None
}

fn load_png_as_tray_icon(path: &str) -> Result<tray_icon::Icon, Box<dyn std::error::Error>> {
    // 使用image crate加载图片
    let img = image::open(path)?;

    // 将图片缩放到16x16像素（状态栏图标标准尺寸）
    let img = img.resize_exact(16, 16, image::imageops::FilterType::Lanczos3);

    // 转换为RGBA格式
    let rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();
    let rgba_data = rgba_img.into_raw();

    // 创建tray-icon的Icon
    let icon = tray_icon::Icon::from_rgba(rgba_data, width, height)?;

    Ok(icon)
}

fn create_fallback_tray_icon() -> tray_icon::Icon {
    info!("🎨 使用程序化生成的状态栏图标（音符图标）");
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
