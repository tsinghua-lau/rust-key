use rodio::{Decoder, OutputStream, Sink};
use serde::{Deserialize, Serialize};

// 引入我们的键盘适配器
mod keyboard_adapter;
use keyboard_adapter::{listen, EventType};

// 引入原生菜单
mod native_menu;
use native_menu::NativeMenu;


use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
// 暂时移除pixels依赖，使用简化的滑动条实现
use chrono::Local;
use log::{debug, error, info, warn};
use simplelog::*;

#[derive(Serialize, Deserialize, Debug)]
struct Settings {
    sound_enabled: bool,
    volume: f32, // 音量范围 0.0 - 1.0
    current_sound: String, // 当前选择的声音文件名
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            sound_enabled: true,
            volume: 0.7, // 默认音量70%
            current_sound: "sound.wav".to_string(), // 默认音效
        }
    }
}

struct AppState {
    settings: Arc<Mutex<Settings>>,
    pub sound_files: Vec<(String, PathBuf)>, // (显示名称, 文件路径) 对
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
        info!("加载的设置: sound_enabled = {}, volume = {:.0}%, current_sound = {}",
              loaded_settings.sound_enabled, loaded_settings.volume * 100.0, loaded_settings.current_sound);
        let settings = Arc::new(Mutex::new(loaded_settings));
        let sound_files = locate_sound_files();
        if sound_files.is_empty() {
            warn!("未找到任何音频文件，请检查assets文件夹");
        } else {
            info!("找到 {} 个音频文件", sound_files.len());
            for (name, path) in &sound_files {
                info!("  - {}: {}", name, path.display());
            }
        }
        Ok(AppState { settings, sound_files })
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

    fn get_current_sound(&self) -> String {
        self.settings.lock().unwrap().current_sound.clone()
    }

    fn set_current_sound(&self, sound_name: &str) {
        let mut settings = self.settings.lock().unwrap();
        settings.current_sound = sound_name.to_string();
        save_settings(&settings);
        info!("声音切换为: {}", sound_name);
    }

    fn get_current_sound_path(&self) -> Option<PathBuf> {
        let current_sound = self.get_current_sound();
        self.sound_files.iter()
            .find(|(name, _)| name == &current_sound)
            .map(|(_, path)| path.clone())
    }
    
    fn play_sound(&self) {
        if !self.is_sound_enabled() {
            debug!("音效已关闭，跳过播放");
            return;
        }
        let sound_path = self.get_current_sound_path();
        if sound_path.is_none() {
            warn!("未找到当前选择的音频文件，取消播放");
            return;
        }
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

    info!("MacOS Key Sound - 启动中...");

    // 首先初始化 Cocoa 应用（必须在主线程）
    unsafe {
        use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicyAccessory};
        use cocoa::base::nil;
        use cocoa::foundation::NSAutoreleasePool;
        use objc::{msg_send, sel, sel_impl};

        let _pool = NSAutoreleasePool::new(nil);

        let app = NSApplication::sharedApplication(nil);
        app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);
        app.finishLaunching();

        info!("应用激活策略已设置为 Accessory");
    }

    let app_state = Arc::new(AppState::new()?);

    // 启动键盘监听线程
    let app_state_for_keyboard = Arc::clone(&app_state);
    thread::spawn(move || {
        info!("键盘监听线程已启动 - 监听并播放音效");

        let listen_result = listen(move |event| {
            if let EventType::KeyPress(key) = &event.event_type {
                info!("按下按键: {:?}", key);
                app_state_for_keyboard.play_sound();
            }
        });

        match listen_result {
            Ok(_) => {
                info!("键盘监听正常结束");
            }
            Err(error) => {
                error!("键盘监听错误: {:?}", error);
                error!("请检查辅助功能权限！");
                error!("解决方案：系统偏好设置 → 安全性与隐私 → 隐私 → 辅助功能");
            }
        }

        info!("键盘监听线程结束");
    });

    info!("应用已启动，请查看系统托盘图标");

    // 使用原生 Cocoa API 创建菜单
    unsafe {
        use cocoa::appkit::NSApp;
        use cocoa::base::{nil, id};
        use objc::{msg_send, sel, sel_impl};

        // 创建原生菜单
        let mut native_menu = NativeMenu::new();

        // 设置托盘图标（使用 @2x 尺寸以支持 Retina 显示屏）
        let icon = create_tray_icon();
        native_menu.set_icon(&icon, 36, 36);
        info!("状态栏图标已设置");

        // 创建菜单构建函数 - 每次打开菜单时都会调用
        let app_state_for_menu = Arc::clone(&app_state);
        let menu_builder = Arc::new(Mutex::new(move |menu: id| {
            unsafe {
                info!("菜单构建函数被调用");

                let app_state_ref = &app_state_for_menu;

                // 添加启用音效菜单项
                let toggle_title = if app_state_ref.is_sound_enabled() {
                    "● 启用音效"
                } else {
                    "○ 启用音效"
                };
                info!("准备添加音效切换菜单项: {}", toggle_title);
                let app_state_toggle = Arc::clone(app_state_ref);
                let toggle_callback = Arc::new(Mutex::new(move || {
                    let enabled = app_state_toggle.toggle_sound();
                    info!("音效已{}", if enabled { "启用" } else { "禁用" });
                }));
                let toggle_item = native_menu::create_menu_item_with_callback_static(toggle_title, toggle_callback);
                let _: () = cocoa::appkit::NSMenu::addItem_(menu, toggle_item);

                // 添加分隔符
                let separator = native_menu::create_separator_static();
                let _: () = cocoa::appkit::NSMenu::addItem_(menu, separator);

                // 添加所有声音选项
                let current_sound = app_state_ref.get_current_sound();
                for (sound_name, _) in &app_state_ref.sound_files {
                    let is_current = sound_name == &current_sound;
                    let title = if is_current {
                        format!("● {}", sound_name)
                    } else {
                        format!("○ {}", sound_name)
                    };
                    let app_state_sound = Arc::clone(app_state_ref);
                    let sound_name_clone = sound_name.clone();
                    let sound_callback = Arc::new(Mutex::new(move || {
                        app_state_sound.set_current_sound(&sound_name_clone);
                        info!("音效已切换到: {}", sound_name_clone);
                    }));
                    let sound_item = native_menu::create_menu_item_with_callback_static(&title, sound_callback);
                    let _: () = cocoa::appkit::NSMenu::addItem_(menu, sound_item);
                }

                let separator2 = native_menu::create_separator_static();
                let _: () = cocoa::appkit::NSMenu::addItem_(menu, separator2);

                // 创建带滑块的音量菜单项
                let app_state_clone = Arc::clone(app_state_ref);
                let volume_callback = Arc::new(Mutex::new(move |volume: f32| {
                    info!("音量通过滑块调整为: {:.0}%", volume * 100.0);
                    app_state_clone.set_volume(volume);
                }));

                let volume_slider_item = native_menu::create_volume_slider_item_static(
                    app_state_ref.get_volume(),
                    volume_callback,
                );
                let _: () = cocoa::appkit::NSMenu::addItem_(menu, volume_slider_item);

                let separator3 = native_menu::create_separator_static();
                let _: () = cocoa::appkit::NSMenu::addItem_(menu, separator3);

                // 添加退出菜单项
                let quit_callback = Arc::new(Mutex::new(|| {
                    info!("用户请求退出应用");
                    unsafe {
                        use cocoa::appkit::NSApp;
                        let app = NSApp();
                        let _: () = msg_send![app, terminate:nil];
                    }
                }));
                let quit_item = native_menu::create_menu_item_with_callback_static("退出", quit_callback);
                let _: () = cocoa::appkit::NSMenu::addItem_(menu, quit_item);
            }
        }));

        // 设置动态菜单
        native_menu.set_dynamic_menu(menu_builder);

        // 运行应用 - 使用 msg_send 调用 run 方法
        let app = NSApp();
        info!("进入主事件循环");
        let _: () = msg_send![app, run];
    }

    Ok(())
}

fn create_tray_icon() -> Vec<u8> {
    // 尝试从文件加载图标，如果失败则使用程序化生成的后备图标
    if let Some(icon_data) = load_tray_icon_from_file() {
        return icon_data;
    }

    // 后备方案：程序化生成图标
    create_fallback_tray_icon()
}

fn load_tray_icon_from_file() -> Option<Vec<u8>> {
    // 构建多个可能的图标路径
    let mut icon_paths = Vec::new();

    // 1. 开发环境路径（优先使用 @2x Retina 图标）
    icon_paths.push("assets/key-icon-tray@2x.png".to_string());

    // 2. macOS应用包中的路径
    if let Ok(exe) = std::env::current_exe() {
        if let Some(resources) = exe.parent() // MacOS 目录
            .and_then(|p| p.parent()) // Contents 目录
            .map(|c| c.join("Resources")) {

            let app_icon_paths = [
                resources.join("assets").join("key-icon-tray@2x.png"),
                resources.join("key-icon-tray@2x.png"), // 直接在Resources下
                resources.join("assets").join("key-icon.png"), // 后备方案
            ];

            for path in &app_icon_paths {
                icon_paths.push(path.to_string_lossy().to_string());
            }
        }
    }

    for path in &icon_paths {
        if std::path::Path::new(path).exists() {
            info!("找到状态栏图标文件: {}", path);

            match load_png_icon(path) {
                Ok(icon_data) => {
                    info!("成功从文件加载状态栏图标: {}", path);
                    return Some(icon_data);
                }
                Err(e) => {
                    warn!("加载状态栏图标失败 {}: {}", path, e);
                }
            }
        }
    }

    info!("未找到状态栏图标文件，使用程序化生成的图标");
    None
}

fn load_png_icon(path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // 使用image crate加载图片
    let img = image::open(path)?;

    // 将图片缩放到36x36像素（@2x Retina 状态栏图标标准尺寸）
    let img = img.resize_exact(36, 36, image::imageops::FilterType::Lanczos3);

    // 转换为RGBA格式
    let rgba_img = img.to_rgba8();
    let rgba_data = rgba_img.into_raw();

    Ok(rgba_data)
}

fn create_fallback_tray_icon() -> Vec<u8> {
    info!("使用改进的程序化状态栏图标");
    // 创建一个18x18像素的改进版音符图标
    let mut rgba = vec![0u8; 18 * 18 * 4]; // 18x18 RGBA

    // 使用更精致的音符图标设计（按比例放大）
    for y in 0..18 {
        for x in 0..18 {
            let idx = (y * 18 + x) * 4;

            // 绘制音符的竖线 (x=10-11, y=3-15，加粗)
            if (x == 10 || x == 11) && y >= 3 && y <= 15 {
                rgba[idx] = 255;     // R
                rgba[idx + 1] = 255; // G
                rgba[idx + 2] = 255; // B
                rgba[idx + 3] = 255; // A
            }
            // 绘制音符的符头 (椭圆形, 底部，更饱满)
            else if ((x >= 6 && x <= 12) && y == 14) ||
                    ((x >= 5 && x <= 13) && y == 15) ||
                    ((x >= 6 && x <= 12) && y == 16) {
                rgba[idx] = 255;     // R
                rgba[idx + 1] = 255; // G
                rgba[idx + 2] = 255; // B
                rgba[idx + 3] = 255; // A
            }
            // 绘制音符的符尾 (顶部的弧线，更流畅)
            else if ((x >= 11 && x <= 14) && y == 3) ||
                    ((x >= 12 && x <= 15) && y == 4) ||
                    (x == 15 && (y == 5 || y == 6)) {
                rgba[idx] = 255;     // R
                rgba[idx + 1] = 255; // G
                rgba[idx + 2] = 255; // B
                rgba[idx + 3] = 255; // A
            }
            // 添加装饰性的小点（五线谱风格）
            else if (x == 2 || x == 3) && (y == 6 || y == 8 || y == 10) {
                rgba[idx] = 200;     // R - 稍微暗一点
                rgba[idx + 1] = 200; // G
                rgba[idx + 2] = 200; // B
                rgba[idx + 3] = 180; // A - 半透明
            }
        }
    }

    rgba
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

fn locate_sound_files() -> Vec<(String, PathBuf)> {
    let mut sound_files = Vec::new();
    let mut asset_dirs = Vec::new();

    // 1. 开发环境：工作目录中的 assets/
    if let Ok(cwd) = std::env::current_dir() {
        asset_dirs.push(cwd.join("assets"));
    } else {
        asset_dirs.push(PathBuf::from("assets"));
    }

    // 2. macOS 应用包中的资源路径
    if let Ok(exe) = std::env::current_exe() {
        debug!("可执行文件路径: {}", exe.display());

        // 方案A: Contents/Resources/assets/ (标准 macOS 应用包结构)
        if let Some(resources) = exe.parent() // MacOS 目录
            .and_then(|p| p.parent()) // Contents 目录
            .map(|c| c.join("Resources").join("assets")) {
            asset_dirs.push(resources.clone());
            debug!("候选assets目录A: {}", resources.display());
        }

        // 方案B: Contents/Resources/ (直接放在Resources下)
        if let Some(resources) = exe.parent() // MacOS 目录
            .and_then(|p| p.parent()) // Contents 目录
            .map(|c| c.join("Resources")) {
            asset_dirs.push(resources.clone());
            debug!("候选assets目录B: {}", resources.display());
        }

        // 方案C: 与可执行文件同目录
        if let Some(exe_dir) = exe.parent() {
            asset_dirs.push(exe_dir.join("assets"));
            debug!("候选assets目录C: {}", exe_dir.join("assets").display());
        }
    }

    // 扫描每个可能的assets目录
    for assets_dir in &asset_dirs {
        if assets_dir.exists() && assets_dir.is_dir() {
            info!("扫描音频目录: {}", assets_dir.display());

            if let Ok(entries) = std::fs::read_dir(assets_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension() {
                            let ext_str = ext.to_string_lossy().to_lowercase();
                            if ext_str == "wav" || ext_str == "mp3" || ext_str == "m4a" || ext_str == "flac" {
                                if let Some(filename) = path.file_name() {
                                    let display_name = filename.to_string_lossy().to_string();
                                    info!("  找到音频文件: {}", display_name);
                                    sound_files.push((display_name, path.clone()));
                                }
                            }
                        }
                    }
                }
            }

            // 如果在这个目录找到了音频文件，就不继续搜索其他目录
            if !sound_files.is_empty() {
                break;
            }
        }
    }

    if sound_files.is_empty() {
        error!("未找到任何音频文件");
    } else {
        info!("总共找到 {} 个音频文件", sound_files.len());
    }

    sound_files
}
