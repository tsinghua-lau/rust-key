use cocoa::appkit::{NSImage, NSMenu, NSMenuItem, NSStatusBar, NSView, NSVariableStatusItemLength};
use cocoa::base::{id, nil, YES, NO};
use cocoa::foundation::{NSString, NSRect, NSPoint, NSSize};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use std::sync::{Arc, Mutex};
use log::debug;

// 音量变化回调类型
pub type VolumeCallback = Arc<Mutex<dyn Fn(f32) + Send>>;
// 菜单项点击回调类型
pub type MenuCallback = Arc<Mutex<dyn Fn() + Send>>;
// 菜单构建器回调类型（直接对传入菜单进行构建）
pub type MenuBuilder = Arc<Mutex<dyn Fn(id) + Send>>;

// 创建原生状态栏菜单
pub struct NativeMenu {
    status_item: id,
    volume_callback: Option<VolumeCallback>,
    menu_delegate: Option<id>, // 保持对委托的强引用
}

impl NativeMenu {
    pub fn new() -> Self {
        unsafe {
            let status_bar = NSStatusBar::systemStatusBar(nil);
            let status_item = status_bar.statusItemWithLength_(NSVariableStatusItemLength);

            NativeMenu {
                status_item,
                volume_callback: None,
                menu_delegate: None,
            }
        }
    }

    // 设置托盘图标 (从RGBA原始数据创建)
    pub fn set_icon(&mut self, icon_data: &[u8], width: u32, height: u32) {
        unsafe {
            // 使用 NSBitmapImageRep 从 RGBA 数据创建图像
            let bitmap_class = class!(NSBitmapImageRep);
            let bitmap: id = msg_send![bitmap_class, alloc];

            // initWithBitmapDataPlanes:pixelsWide:pixelsHigh:bitsPerSample:samplesPerPixel:hasAlpha:isPlanar:colorSpaceName:bytesPerRow:bitsPerPixel:
            let color_space = NSString::alloc(nil).init_str("NSDeviceRGBColorSpace");
            let bytes_per_row = width * 4; // RGBA = 4 bytes per pixel
            let bits_per_pixel = 32; // 8 bits per component * 4 components

            let bitmap: id = msg_send![
                bitmap,
                initWithBitmapDataPlanes:std::ptr::null_mut::<*mut u8>()
                pixelsWide:width as i64
                pixelsHigh:height as i64
                bitsPerSample:8
                samplesPerPixel:4
                hasAlpha:YES
                isPlanar:NO
                colorSpaceName:color_space
                bytesPerRow:bytes_per_row as i64
                bitsPerPixel:bits_per_pixel as i64
            ];

            if bitmap.is_null() {
                debug!("Failed to create NSBitmapImageRep");
                return;
            }

            // 复制 RGBA 数据到 bitmap
            let bitmap_data: *mut u8 = msg_send![bitmap, bitmapData];
            if !bitmap_data.is_null() {
                std::ptr::copy_nonoverlapping(
                    icon_data.as_ptr(),
                    bitmap_data,
                    icon_data.len()
                );
            }

            // 创建 NSImage 并添加 bitmap representation
            let ns_image = NSImage::alloc(nil);
            let size = NSSize::new(width as f64, height as f64);
            let ns_image: id = msg_send![ns_image, initWithSize:size];

            if ns_image.is_null() {
                debug!("Failed to create NSImage");
                return;
            }

            let _: () = msg_send![ns_image, addRepresentation:bitmap];

            // 设置图标大小为状态栏标准尺寸
            let status_bar_size = NSSize::new(18.0, 18.0);
            let _: () = msg_send![ns_image, setSize:status_bar_size];

            // 设置为模板图片，这样会自动适配暗黑模式
            let _: () = msg_send![ns_image, setTemplate:YES];

            let button: id = msg_send![self.status_item, button];
            if !button.is_null() {
                let _: () = msg_send![button, setImage:ns_image];
                debug!("Status bar icon set successfully");
            } else {
                debug!("Failed to get status item button");
            }
        }
    }

    // 设置音量变化回调
    pub fn set_volume_callback<F>(&mut self, callback: F)
    where
        F: Fn(f32) + Send + 'static,
    {
        self.volume_callback = Some(Arc::new(Mutex::new(callback)));
    }

    // 创建菜单
    pub fn create_menu(&self, current_volume: f32) -> id {
        unsafe {
            let menu = NSMenu::alloc(nil);
            let menu = menu.init();
            let _: () = msg_send![menu, setAutoenablesItems:NO];

            menu
        }
    }

    // 创建带滑块的音量菜单项
    pub fn create_volume_slider_item(&self, initial_volume: f32, callback: VolumeCallback) -> id {
        unsafe {
            let menu_item = NSMenuItem::alloc(nil);
            let menu_item = menu_item.init();

            // 创建容器视图（增加宽度以容纳更宽的百分比标签和右边距）
            let container_view = NSView::alloc(nil);
            let container_frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(200.0, 50.0));
            let container_view: id = msg_send![container_view, initWithFrame:container_frame];

            // 创建音量标签
            let label_class = class!(NSTextField);
            let label: id = msg_send![label_class, alloc];
            let label_frame = NSRect::new(NSPoint::new(15.0, 25.0), NSSize::new(60.0, 20.0));
            let label: id = msg_send![label, initWithFrame:label_frame];

            let label_text = NSString::alloc(nil).init_str("音量:");
            let _: () = msg_send![label, setStringValue:label_text];
            let _: () = msg_send![label, setBordered:NO];
            let _: () = msg_send![label, setDrawsBackground:NO];
            let _: () = msg_send![label, setEditable:NO];
            let _: () = msg_send![label, setSelectable:NO];

            let _: () = msg_send![container_view, addSubview:label];

            // 创建百分比标签（增加宽度以完整显示百分比符号，调整位置增加右边距）
            let percent_label: id = msg_send![label_class, alloc];
            let percent_frame = NSRect::new(NSPoint::new(135.0, 25.0), NSSize::new(50.0, 20.0));
            let percent_label: id = msg_send![percent_label, initWithFrame:percent_frame];

            let percent_text = NSString::alloc(nil).init_str(&format!("{:.0}%", initial_volume * 100.0));
            let _: () = msg_send![percent_label, setStringValue:percent_text];
            let _: () = msg_send![percent_label, setBordered:NO];
            let _: () = msg_send![percent_label, setDrawsBackground:NO];
            let _: () = msg_send![percent_label, setEditable:NO];
            let _: () = msg_send![percent_label, setSelectable:NO];
            let _: () = msg_send![percent_label, setAlignment:2]; // NSTextAlignmentRight

            // 给标签设置一个tag，方便后续更新
            let _: () = msg_send![percent_label, setTag:999];

            let _: () = msg_send![container_view, addSubview:percent_label];

            // 创建滑块 - 使用 objc 直接创建 NSSlider（增加宽度以匹配容器）
            let slider_class = class!(NSSlider);
            let slider: id = msg_send![slider_class, alloc];
            let slider_frame = NSRect::new(NSPoint::new(15.0, 5.0), NSSize::new(170.0, 20.0));
            let slider: id = msg_send![slider, initWithFrame:slider_frame];

            let _: () = msg_send![slider, setMinValue:0.0f64];
            let _: () = msg_send![slider, setMaxValue:100.0f64];
            let _: () = msg_send![slider, setDoubleValue:(initial_volume * 100.0) as f64];
            let _: () = msg_send![slider, setContinuous:YES];

            // 创建滑块回调类
            let slider_delegate_class = create_slider_delegate_class();
            let slider_delegate: id = msg_send![slider_delegate_class, alloc];
            let slider_delegate: id = msg_send![slider_delegate, init];

            // 将回调和标签保存到delegate中
            let callback_ptr = Box::into_raw(Box::new(callback)) as *mut std::ffi::c_void;
            (*slider_delegate).set_ivar("callback", callback_ptr);
            (*slider_delegate).set_ivar("percentLabel", percent_label);

            // 设置滑块的target和action
            let _: () = msg_send![slider, setTarget:slider_delegate];
            let _: () = msg_send![slider, setAction:sel!(sliderValueChanged:)];

            let _: () = msg_send![container_view, addSubview:slider];

            // 将视图设置到菜单项
            let _: () = msg_send![menu_item, setView:container_view];

            menu_item
        }
    }

    // 创建普通文本菜单项
    pub fn create_menu_item(&self, title: &str, action: Option<Sel>) -> id {
        unsafe {
            let menu_item = NSMenuItem::alloc(nil);
            let title_ns = NSString::alloc(nil).init_str(title);
            let empty_string = NSString::alloc(nil).init_str("");

            let menu_item = if let Some(sel) = action {
                NSMenuItem::initWithTitle_action_keyEquivalent_(menu_item, title_ns, sel, empty_string)
            } else {
                // 使用 NULL selector (0 as Sel)
                let null_selector: Sel = std::mem::transmute(0usize);
                NSMenuItem::initWithTitle_action_keyEquivalent_(menu_item, title_ns, null_selector, empty_string)
            };

            menu_item
        }
    }

    // 创建带回调的菜单项
    pub fn create_menu_item_with_callback(&self, title: &str, callback: MenuCallback) -> id {
        unsafe {
            let menu_item = NSMenuItem::alloc(nil);
            let title_ns = NSString::alloc(nil).init_str(title);
            let empty_string = NSString::alloc(nil).init_str("");

            // 创建菜单项委托
            let delegate_class = create_menu_item_delegate_class();
            let delegate: id = msg_send![delegate_class, alloc];
            let delegate: id = msg_send![delegate, init];

            // 保存回调
            let callback_ptr = Box::into_raw(Box::new(callback)) as *mut std::ffi::c_void;
            (*delegate).set_ivar("callback", callback_ptr);

            // 创建菜单项并设置target和action
            let menu_item = NSMenuItem::initWithTitle_action_keyEquivalent_(
                menu_item, 
                title_ns, 
                sel!(menuItemClicked:), 
                empty_string
            );
            let _: () = msg_send![menu_item, setTarget:delegate];

            menu_item
        }
    }

    // 创建分隔符
    pub fn create_separator(&self) -> id {
        unsafe {
            NSMenuItem::separatorItem(nil)
        }
    }

    // 设置菜单
    pub fn set_menu(&mut self, menu: id) {
        unsafe {
            let _: () = msg_send![self.status_item, setMenu:menu];
        }
    }

    // 设置动态菜单（每次打开菜单时重新构建）
    pub fn set_dynamic_menu(&mut self, menu_builder: MenuBuilder) {
        unsafe {
            // 创建一个空菜单作为占位
            let placeholder_menu = NSMenu::alloc(nil);
            let placeholder_menu = placeholder_menu.init();

            // 创建菜单委托类
            let delegate_class = create_menu_delegate_class();
            let delegate: id = msg_send![delegate_class, alloc];
            let delegate: id = msg_send![delegate, init];

            // 保存菜单构建器
            let builder_ptr = Box::into_raw(Box::new(menu_builder)) as *mut std::ffi::c_void;
            (*delegate).set_ivar("menuBuilder", builder_ptr);

            // 设置菜单委托
            let _: () = msg_send![placeholder_menu, setDelegate:delegate];

            // 设置菜单到状态项
            let _: () = msg_send![self.status_item, setMenu:placeholder_menu];

            // 保持对委托的强引用，防止被释放
            let _: () = msg_send![delegate, retain];
            self.menu_delegate = Some(delegate);

            debug!("动态菜单已设置，委托已保留");
        }
    }
}

// 静态函数版本 - 不需要 self

pub fn create_menu_static() -> id {
    unsafe {
        let menu = NSMenu::alloc(nil);
        let menu = menu.init();
        let _: () = msg_send![menu, setAutoenablesItems:NO];
        menu
    }
}

pub fn create_menu_item_static(title: &str, action: Option<Sel>) -> id {
    unsafe {
        let menu_item = NSMenuItem::alloc(nil);
        let title_ns = NSString::alloc(nil).init_str(title);
        let empty_string = NSString::alloc(nil).init_str("");

        let menu_item = if let Some(sel) = action {
            NSMenuItem::initWithTitle_action_keyEquivalent_(menu_item, title_ns, sel, empty_string)
        } else {
            let null_selector: Sel = std::mem::transmute(0usize);
            NSMenuItem::initWithTitle_action_keyEquivalent_(menu_item, title_ns, null_selector, empty_string)
        };

        menu_item
    }
}

pub fn create_menu_item_with_callback_static(title: &str, callback: MenuCallback) -> id {
    unsafe {
        let menu_item = NSMenuItem::alloc(nil);
        let title_ns = NSString::alloc(nil).init_str(title);
        let empty_string = NSString::alloc(nil).init_str("");

        // 创建菜单项委托
        let delegate_class = create_menu_item_delegate_class();
        let delegate: id = msg_send![delegate_class, alloc];
        let delegate: id = msg_send![delegate, init];

        // 保存回调
        let callback_ptr = Box::into_raw(Box::new(callback)) as *mut std::ffi::c_void;
        (*delegate).set_ivar("callback", callback_ptr);

        // 创建菜单项并设置target和action
        let menu_item = NSMenuItem::initWithTitle_action_keyEquivalent_(
            menu_item,
            title_ns,
            sel!(menuItemClicked:),
            empty_string
        );
        let _: () = msg_send![menu_item, setTarget:delegate];

        // 如果标题包含 ●，将其设置为紫色
        if title.contains("●") {
            set_attributed_title_with_purple_circle(menu_item, title);
        }

        menu_item
    }
}

// 为菜单项设置带紫色圆圈的 AttributedString
fn set_attributed_title_with_purple_circle(menu_item: id, title: &str) {
    unsafe {
        // 创建 NSMutableAttributedString
        let attr_string_class = class!(NSMutableAttributedString);
        let attr_string: id = msg_send![attr_string_class, alloc];
        let title_ns = NSString::alloc(nil).init_str(title);
        let attr_string: id = msg_send![attr_string, initWithString:title_ns];

        // 找到 ● 的位置
        if let Some(circle_pos) = title.find("●") {
            // 创建紫色 (RGB: 147, 51, 234)
            let color_class = class!(NSColor);
            let purple_color: id = msg_send![color_class,
                colorWithCalibratedRed:147.0/255.0
                green:51.0/255.0
                blue:234.0/255.0
                alpha:1.0
            ];

            // 创建属性字典
            let dict_class = class!(NSMutableDictionary);
            let color_dict: id = msg_send![dict_class, dictionary];
            let foreground_color_key = NSString::alloc(nil).init_str("NSColor");
            let _: () = msg_send![color_dict, setObject:purple_color forKey:foreground_color_key];

            // 应用紫色到圆圈字符
            let range = cocoa::foundation::NSRange::new(circle_pos as u64, 1);
            let _: () = msg_send![attr_string,
                addAttribute:foreground_color_key
                value:purple_color
                range:range
            ];
        }

        // 设置 attributed title
        let _: () = msg_send![menu_item, setAttributedTitle:attr_string];
    }
}

pub fn create_separator_static() -> id {
    unsafe {
        NSMenuItem::separatorItem(nil)
    }
}

pub fn create_volume_slider_item_static(initial_volume: f32, callback: VolumeCallback) -> id {
    unsafe {
        let menu_item = NSMenuItem::alloc(nil);
        let menu_item = menu_item.init();

        // 创建容器视图
        let container_view = NSView::alloc(nil);
        let container_frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(150.0, 50.0));
        let container_view: id = msg_send![container_view, initWithFrame:container_frame];

        // 创建音量标签
        let label_class = class!(NSTextField);
        let label: id = msg_send![label_class, alloc];
        let label_frame = NSRect::new(NSPoint::new(15.0, 25.0), NSSize::new(60.0, 20.0));
        let label: id = msg_send![label, initWithFrame:label_frame];

        let label_text = NSString::alloc(nil).init_str("音量:");
        let _: () = msg_send![label, setStringValue:label_text];
        let _: () = msg_send![label, setBordered:NO];
        let _: () = msg_send![label, setDrawsBackground:NO];
        let _: () = msg_send![label, setEditable:NO];
        let _: () = msg_send![label, setSelectable:NO];

        let _: () = msg_send![container_view, addSubview:label];

        // 创建百分比标签
        let percent_label: id = msg_send![label_class, alloc];
        let percent_frame = NSRect::new(NSPoint::new(115.0, 25.0), NSSize::new(30.0, 20.0));
        let percent_label: id = msg_send![percent_label, initWithFrame:percent_frame];

        let percent_text = NSString::alloc(nil).init_str(&format!("{:.0}%", initial_volume * 100.0));
        let _: () = msg_send![percent_label, setStringValue:percent_text];
        let _: () = msg_send![percent_label, setBordered:NO];
        let _: () = msg_send![percent_label, setDrawsBackground:NO];
        let _: () = msg_send![percent_label, setEditable:NO];
        let _: () = msg_send![percent_label, setSelectable:NO];
        let _: () = msg_send![percent_label, setAlignment:2]; // NSTextAlignmentRight

        // 给标签设置一个tag，方便后续更新
        let _: () = msg_send![percent_label, setTag:999];

        let _: () = msg_send![container_view, addSubview:percent_label];

        // 创建滑块 - 使用 objc 直接创建 NSSlider
        let slider_class = class!(NSSlider);
        let slider: id = msg_send![slider_class, alloc];
        let slider_frame = NSRect::new(NSPoint::new(15.0, 5.0), NSSize::new(125.0, 20.0));
        let slider: id = msg_send![slider, initWithFrame:slider_frame];

        let _: () = msg_send![slider, setMinValue:0.0f64];
        let _: () = msg_send![slider, setMaxValue:100.0f64];
        let _: () = msg_send![slider, setDoubleValue:(initial_volume * 100.0) as f64];
        let _: () = msg_send![slider, setContinuous:YES];

        // 创建滑块回调类
        let slider_delegate_class = create_slider_delegate_class();
        let slider_delegate: id = msg_send![slider_delegate_class, alloc];
        let slider_delegate: id = msg_send![slider_delegate, init];

        // 将回调和标签保存到delegate中
        let callback_ptr = Box::into_raw(Box::new(callback)) as *mut std::ffi::c_void;
        (*slider_delegate).set_ivar("callback", callback_ptr);
        (*slider_delegate).set_ivar("percentLabel", percent_label);

        // 设置滑块的target和action
        let _: () = msg_send![slider, setTarget:slider_delegate];
        let _: () = msg_send![slider, setAction:sel!(sliderValueChanged:)];

        let _: () = msg_send![container_view, addSubview:slider];

        // 将视图设置到菜单项
        let _: () = msg_send![menu_item, setView:container_view];

        menu_item
    }
}

// 创建菜单委托类
fn create_menu_delegate_class() -> &'static Class {
    static mut MENU_DELEGATE_CLASS: Option<&'static Class> = None;
    static INIT: std::sync::Once = std::sync::Once::new();

    INIT.call_once(|| unsafe {
        let superclass = class!(NSObject);
        let mut decl = ClassDecl::new("MenuDelegate", superclass).unwrap();

        // 添加实例变量来存储菜单构建器
        decl.add_ivar::<*mut std::ffi::c_void>("menuBuilder");

        // 添加菜单需要更新时的方法 - NSMenuDelegate 协议
        extern "C" fn menu_needs_update(this: &mut Object, _cmd: Sel, menu: id) {
            use log::{error, info};
            if let Err(err) = std::panic::catch_unwind(|| unsafe {
                info!("菜单需要更新，开始构建菜单");
                let builder_ptr: *mut std::ffi::c_void = *this.get_ivar("menuBuilder");

                if builder_ptr.is_null() {
                    info!("菜单构建器指针为空");
                    return;
                }

                info!("获取到菜单构建器指针");
                let builder = &*(builder_ptr as *const MenuBuilder);
                match builder.lock() {
                    Ok(build_fn) => {
                        let _: () = msg_send![menu, removeAllItems];
                        info!("已清空菜单项，开始调用菜单构建函数");
                        build_fn(menu);
                        info!("菜单更新完成");
                    }
                    Err(_) => info!("无法获取菜单构建器锁"),
                }
            }) {
                let panic_msg = if let Some(msg) = err.downcast_ref::<&str>() {
                    *msg
                } else if let Some(msg) = err.downcast_ref::<String>() {
                    msg.as_str()
                } else {
                    "未知 panic"
                };
                error!("菜单更新过程中发生panic: {}", panic_msg);
            }
        }

        unsafe {
            decl.add_method(
                sel!(menuNeedsUpdate:),
                menu_needs_update as extern "C" fn(&mut Object, Sel, id),
            );
        }

        MENU_DELEGATE_CLASS = Some(decl.register());
    });

    unsafe { MENU_DELEGATE_CLASS.unwrap() }
}

// 创建菜单项委托类
fn create_menu_item_delegate_class() -> &'static Class {
    static mut MENU_ITEM_DELEGATE_CLASS: Option<&'static Class> = None;
    static INIT: std::sync::Once = std::sync::Once::new();

    INIT.call_once(|| unsafe {
        let superclass = class!(NSObject);
        let mut decl = ClassDecl::new("MenuItemDelegate", superclass).unwrap();

        // 添加实例变量来存储回调
        decl.add_ivar::<*mut std::ffi::c_void>("callback");

        // 添加菜单项点击的方法
        extern "C" fn menu_item_clicked(this: &mut Object, _cmd: Sel, _sender: id) {
            unsafe {
                let callback_ptr: *mut std::ffi::c_void = *this.get_ivar("callback");
                if !callback_ptr.is_null() {
                    let callback = &*(callback_ptr as *const MenuCallback);
                    if let Ok(cb) = callback.lock() {
                        cb();
                    }
                }
            }
        }

        unsafe {
            decl.add_method(
                sel!(menuItemClicked:),
                menu_item_clicked as extern "C" fn(&mut Object, Sel, id),
            );
        }

        MENU_ITEM_DELEGATE_CLASS = Some(decl.register());
    });

    unsafe { MENU_ITEM_DELEGATE_CLASS.unwrap() }
}

// 创建滑块委托类
fn create_slider_delegate_class() -> &'static Class {
    static mut SLIDER_DELEGATE_CLASS: Option<&'static Class> = None;
    static INIT: std::sync::Once = std::sync::Once::new();

    INIT.call_once(|| unsafe {
        let superclass = class!(NSObject);
        let mut decl = ClassDecl::new("VolumeSliderDelegate", superclass).unwrap();

        // 添加实例变量来存储回调
        decl.add_ivar::<*mut std::ffi::c_void>("callback");
        decl.add_ivar::<id>("percentLabel");

        // 添加滑块值变化的方法
        extern "C" fn slider_value_changed(this: &mut Object, _cmd: Sel, slider: id) {
            unsafe {
                let callback_ptr: *mut std::ffi::c_void = *this.get_ivar("callback");
                let percent_label: id = *this.get_ivar("percentLabel");

                if !callback_ptr.is_null() {
                    let callback = &*(callback_ptr as *const VolumeCallback);

                    let value: f64 = msg_send![slider, doubleValue];
                    let volume = (value / 100.0) as f32;

                    debug!("滑块值变化: {:.0}%", value);

                    // 更新百分比标签
                    if !percent_label.is_null() {
                        let percent_text = NSString::alloc(nil).init_str(&format!("{:.0}%", value));
                        let _: () = msg_send![percent_label, setStringValue:percent_text];
                        let _: () = msg_send![percent_label, setNeedsDisplay:YES];
                        let _: () = msg_send![percent_label, displayIfNeeded];
                    }

                    // 调用回调
                    if let Ok(cb) = callback.lock() {
                        cb(volume);
                    }
                }
            }
        }

        unsafe {
            decl.add_method(
                sel!(sliderValueChanged:),
                slider_value_changed as extern "C" fn(&mut Object, Sel, id),
            );
        }

        SLIDER_DELEGATE_CLASS = Some(decl.register());
    });

    unsafe { SLIDER_DELEGATE_CLASS.unwrap() }
}


impl Drop for NativeMenu {
    fn drop(&mut self) {
        unsafe {
            // 释放委托
            if let Some(delegate) = self.menu_delegate {
                let _: () = msg_send![delegate, release];
            }
        }
    }
}
