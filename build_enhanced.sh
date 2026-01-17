#!/bin/bash

# AmpKey - 增强版一键打包构建脚本
# 自动完成编译、权限配置和DMG打包的完整流程

set -e  # 遇到错误立即退出

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 应用信息
APP_NAME="AmpKey"
VERSION="0.1.0"
BUNDLE_PATH="target/release/bundle/osx/${APP_NAME}.app"
INFO_PLIST="${BUNDLE_PATH}/Contents/Info.plist"
DIST_DIR="dist"
DMG_NAME="${APP_NAME} ${VERSION}.dmg"

# 函数：打印状态消息
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 函数：检查命令是否存在
check_command() {
    if ! command -v "$1" &> /dev/null; then
        print_error "命令 '$1' 未找到，请先安装"
        return 1
    fi
    return 0
}

# 函数：添加macOS权限配置
add_permissions() {
    print_status "添加macOS权限配置到Info.plist..."

    # 检查Info.plist是否存在
    if [ ! -f "$INFO_PLIST" ]; then
        print_error "Info.plist文件不存在: $INFO_PLIST"
        return 1
    fi

    # 检查是否已经包含权限配置
    if grep -q "NSAccessibilityUsageDescription" "$INFO_PLIST"; then
        print_warning "权限配置已存在，跳过添加"
        return 0
    fi

    # 备份原始文件
    cp "$INFO_PLIST" "$INFO_PLIST.backup"

    # 在</dict>前添加权限配置
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS版本
        sed -i '' '/<\/dict>/i\
  <key>NSAccessibilityUsageDescription</key>\
  <string>此应用需要辅助功能权限以监听全局键盘事件并播放按键音效。</string>\
  <key>NSInputMonitoringUsageDescription</key>\
  <string>此应用需要输入监控权限以检测键盘按键事件。</string>\
  <key>LSUIElement</key>\
  <true/>\
  <key>NSAppleEventsUsageDescription</key>\
  <string>此应用需要访问Apple事件以提供键盘监听功能。</string>
' "$INFO_PLIST"
    else
        # Linux版本 (如果在Linux环境下测试)
        sed -i '/<\/dict>/i\
  <key>NSAccessibilityUsageDescription</key>\
  <string>此应用需要辅助功能权限以监听全局键盘事件并播放按键音效。</string>\
  <key>NSInputMonitoringUsageDescription</key>\
  <string>此应用需要输入监控权限以检测键盘按键事件。</string>\
  <key>LSUIElement</key>\
  <true/>\
  <key>NSAppleEventsUsageDescription</key>\
  <string>此应用需要访问Apple事件以提供键盘监听功能。</string>
' "$INFO_PLIST"
    fi

    if [ $? -eq 0 ]; then
        print_success "权限配置已添加"
        return 0
    else
        print_error "权限配置添加失败，恢复备份"
        mv "$INFO_PLIST.backup" "$INFO_PLIST"
        return 1
    fi
}

# 函数：清理旧的构建文件
cleanup() {
    print_status "清理旧的构建文件..."

    if [ -d "$BUNDLE_PATH" ]; then
        rm -rf "$BUNDLE_PATH"
        print_status "删除旧的应用包"
    fi

    if [ -f "${DIST_DIR}/${DMG_NAME}" ]; then
        rm -f "${DIST_DIR}/${DMG_NAME}"
        print_status "删除旧的DMG文件"
    fi

    # 清理所有DMG文件
    find "$DIST_DIR" -name "*.dmg" -type f -delete 2>/dev/null || true
}

# 函数：创建dist目录
create_dist_dir() {
    if [ ! -d "$DIST_DIR" ]; then
        mkdir -p "$DIST_DIR"
        print_status "创建dist目录"
    fi
}

# 函数：创建icns图标文件
create_icns() {
    print_status "创建icns图标文件..."

    local PNG_FILE="assets/app-icon.png"
    local ICNS_FILE="assets/app-icon.icns"
    local ICONSET_DIR="assets/app-icon.iconset"

    # 检查PNG文件是否存在
    if [ ! -f "$PNG_FILE" ]; then
        print_error "PNG图标文件不存在: $PNG_FILE"
        return 1
    fi

    # 如果icns文件已存在且PNG文件未修改，则跳过
    if [ -f "$ICNS_FILE" ] && [ "$PNG_FILE" -ot "$ICNS_FILE" ]; then
        print_status "icns文件已是最新，跳过生成"
        return 0
    fi

    # 检查sips命令（macOS自带的图像处理工具）
    if ! check_command "sips"; then
        print_error "sips命令未找到，无法生成icns文件"
        return 1
    fi

    # 创建iconset目录
    rm -rf "$ICONSET_DIR"
    mkdir -p "$ICONSET_DIR"

    # 生成所需的各种尺寸的图标
    print_status "生成各种尺寸的图标..."
    sips -z 16 16     "$PNG_FILE" --out "${ICONSET_DIR}/icon_16x16.png" > /dev/null 2>&1
    sips -z 32 32     "$PNG_FILE" --out "${ICONSET_DIR}/icon_16x16@2x.png" > /dev/null 2>&1
    sips -z 32 32     "$PNG_FILE" --out "${ICONSET_DIR}/icon_32x32.png" > /dev/null 2>&1
    sips -z 64 64     "$PNG_FILE" --out "${ICONSET_DIR}/icon_32x32@2x.png" > /dev/null 2>&1
    sips -z 128 128   "$PNG_FILE" --out "${ICONSET_DIR}/icon_128x128.png" > /dev/null 2>&1
    sips -z 256 256   "$PNG_FILE" --out "${ICONSET_DIR}/icon_128x128@2x.png" > /dev/null 2>&1
    sips -z 256 256   "$PNG_FILE" --out "${ICONSET_DIR}/icon_256x256.png" > /dev/null 2>&1
    sips -z 512 512   "$PNG_FILE" --out "${ICONSET_DIR}/icon_256x256@2x.png" > /dev/null 2>&1
    sips -z 512 512   "$PNG_FILE" --out "${ICONSET_DIR}/icon_512x512.png" > /dev/null 2>&1
    sips -z 1024 1024 "$PNG_FILE" --out "${ICONSET_DIR}/icon_512x512@2x.png" > /dev/null 2>&1

    # 将iconset转换为icns
    print_status "将iconset转换为icns..."
    if iconutil -c icns "$ICONSET_DIR" -o "$ICNS_FILE"; then
        print_success "icns文件生成成功: $ICNS_FILE"
        # 清理临时目录
        rm -rf "$ICONSET_DIR"
        return 0
    else
        print_error "icns文件生成失败"
        rm -rf "$ICONSET_DIR"
        return 1
    fi
}

# 函数：创建状态栏图标
create_tray_icon() {
    print_status "创建状态栏图标 (36x36 @2x Retina)..."

    local SOURCE_ICON="assets/key-icon.png"
    local TRAY_ICON="assets/key-icon-tray@2x.png"

    # 检查源图标文件是否存在
    if [ ! -f "$SOURCE_ICON" ]; then
        print_error "源图标文件不存在: $SOURCE_ICON"
        return 1
    fi

    # 如果状态栏图标已存在且源文件未修改，则跳过
    if [ -f "$TRAY_ICON" ] && [ "$SOURCE_ICON" -ot "$TRAY_ICON" ]; then
        print_status "状态栏图标已是最新，跳过生成"
        return 0
    fi

    # 检查sips命令
    if ! check_command "sips"; then
        print_error "sips命令未找到，无法生成状态栏图标"
        return 1
    fi

    # 生成36x36像素的@2x Retina状态栏图标
    print_status "缩放图标到 36x36 像素 (支持 Retina 显示屏)..."
    if sips -z 36 36 "$SOURCE_ICON" --out "$TRAY_ICON" > /dev/null 2>&1; then
        print_success "状态栏图标生成成功: $TRAY_ICON"
        return 0
    else
        print_error "状态栏图标生成失败"
        return 1
    fi
}

# 函数：检查资源文件
check_resources() {
    print_status "检查资源文件..."

    if [ ! -f "assets/sound.wav" ]; then
        print_error "缺少资源文件: assets/sound.wav"
        print_status "请确保音频文件存在后再构建"
        return 1
    fi

    if [ ! -f "assets/app-icon.png" ]; then
        print_error "缺少图标文件: assets/app-icon.png"
        print_status "请确保PNG图标文件存在后再构建"
        return 1
    fi

    if [ ! -f "assets/key-icon.png" ]; then
        print_error "缺少状态栏源图标: assets/key-icon.png"
        print_status "请确保状态栏PNG图标文件存在后再构建"
        return 1
    fi

    print_success "资源文件检查完成"
    return 0
}

# 函数：安装依赖工具
install_dependencies() {
    print_status "检查并安装必要的构建工具..."

    # 检查cargo-bundle
    if ! check_command "cargo-bundle"; then
        print_status "安装cargo-bundle..."
        if cargo install cargo-bundle; then
            print_success "cargo-bundle安装完成"
        else
            print_error "cargo-bundle安装失败"
            return 1
        fi
    fi

    # 检查create-dmg
    if ! check_command "create-dmg"; then
        print_warning "create-dmg未安装"
        print_status "请运行以下命令安装: npm install -g create-dmg"
        print_status "或使用Homebrew: brew install create-dmg"
        return 1
    fi

    return 0
}

# 主构建流程
main() {
    echo "=================================================="
    print_status "🚀 开始 ${APP_NAME} 增强版一键构建流程..."
    echo "=================================================="

    # 1. 检查资源文件
    if ! check_resources; then
        exit 1
    fi

    # 2. 创建icns图标文件
    if ! create_icns; then
        print_warning "icns文件生成失败，但继续构建流程"
    fi

    # 3. 创建状态栏图标
    if ! create_tray_icon; then
        print_warning "状态栏图标生成失败，但继续构建流程"
    fi

    # 4. 检查必要的命令
    print_status "检查必要的构建工具..."
    if ! check_command "cargo"; then
        print_error "Rust/Cargo未安装，请先安装Rust开发环境"
        exit 1
    fi

    # 5. 安装依赖工具
    if ! install_dependencies; then
        print_error "依赖工具安装失败"
        exit 1
    fi

    # 6. 清理旧文件
    cleanup

    # 7. 创建输出目录
    create_dist_dir

    # 8. 激活Rust环境
    print_status "激活Rust环境..."
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    fi

    # 9. Rust编译和打包
    print_status "开始Rust编译和打包..."
    echo "--------------------------------------------------"

    # 先运行clean
    print_status "清理之前的构建..."
    cargo clean

    # 编译release版本
    print_status "编译release版本..."
    if cargo build --release; then
        print_success "编译完成"
    else
        print_error "编译失败"
        exit 1
    fi

    # 创建应用包
    print_status "创建macOS应用包..."
    if cargo bundle --release; then
        print_success "应用包创建完成"
    else
        print_error "应用包创建失败"
        exit 1
    fi

    # 10. 检查应用包是否生成
    if [ ! -d "$BUNDLE_PATH" ]; then
        print_error "应用包未生成: $BUNDLE_PATH"
        exit 1
    fi
    print_success "应用包生成成功: $BUNDLE_PATH"

    # 11. 添加权限配置
    if ! add_permissions; then
        print_error "权限配置添加失败"
        exit 1
    fi

    # 12. 创建DMG安装包
    print_status "创建DMG安装包..."
    echo "--------------------------------------------------"
    if create-dmg --overwrite --no-code-sign "$BUNDLE_PATH" "$DIST_DIR"; then
        print_success "DMG创建完成"
    else
        print_error "DMG创建失败"
        exit 1
    fi

    # 13. 查找实际生成的DMG文件
    ACTUAL_DMG=$(find "$DIST_DIR" -name "*.dmg" -type f | head -1)

    # 14. 显示构建结果
    echo "=================================================="
    print_success "🎉 构建完成！"
    echo ""
    print_status "📦 构建产物:"
    echo "   应用程序: $BUNDLE_PATH"
    if [ -n "$ACTUAL_DMG" ]; then
        echo "   安装包:   $ACTUAL_DMG"
    else
        echo "   安装包:   未找到DMG文件"
    fi
    echo ""

    # 显示文件大小
    if [ -n "$ACTUAL_DMG" ] && [ -f "$ACTUAL_DMG" ]; then
        DMG_SIZE=$(du -h "$ACTUAL_DMG" | cut -f1)
        print_status "📊 安装包大小: $DMG_SIZE"
    fi

    # 显示应用信息
    if [ -f "$INFO_PLIST" ]; then
        BUILD_VERSION=$(grep -A1 "CFBundleVersion" "$INFO_PLIST" | tail -1 | sed 's/.*<string>\(.*\)<\/string>.*/\1/' | xargs)
        print_status "🏷️  构建版本: $BUILD_VERSION"
    fi

    # 显示可执行文件大小
    EXEC_PATH="${BUNDLE_PATH}/Contents/MacOS/macos-key-sound"
    if [ -f "$EXEC_PATH" ]; then
        EXEC_SIZE=$(du -h "$EXEC_PATH" | cut -f1)
        print_status "⚙️  可执行文件大小: $EXEC_SIZE"
    fi

    echo ""
    print_status "🚀 测试命令:"
    echo "   应用包: open \"$BUNDLE_PATH\""
    if [ -n "$ACTUAL_DMG" ]; then
        echo "   安装包: open \"$ACTUAL_DMG\""
    fi
    echo ""
    print_warning "⚠️  重要提醒:"
    echo "   首次运行需要在系统偏好设置中授予辅助功能权限"
    echo "   系统偏好设置 → 安全性与隐私 → 隐私 → 辅助功能"
    echo "=================================================="
}

# 显示帮助信息
show_help() {
    echo "AmpKey - 增强版一键构建脚本"
    echo ""
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  -h, --help     显示此帮助信息"
    echo "  -c, --clean    仅清理构建文件"
    echo "  -v, --verbose  显示详细构建信息"
    echo "  -r, --resources 检查资源文件"
    echo ""
    echo "功能:"
    echo "  • 自动编译Rust代码"
    echo "  • 创建macOS应用包和icns图标"
    echo "  • 生成状态栏图标 (36x36 @2x Retina)"
    echo "  • 自动添加macOS权限配置"
    echo "  • 生成DMG安装包"
    echo "  • 完整的错误处理"
    echo ""
    echo "示例:"
    echo "  $0              # 执行完整构建"
    echo "  $0 --clean     # 清理构建文件"
    echo "  $0 --verbose   # 详细模式构建"
}

# 参数解析
case "${1:-}" in
    -h|--help)
        show_help
        exit 0
        ;;
    -c|--clean)
        print_status "清理模式..."
        cleanup
        # 额外清理cargo构建文件
        if check_command "cargo"; then
            cargo clean
        fi
        print_success "清理完成"
        exit 0
        ;;
    -v|--verbose)
        set -x  # 启用详细模式
        main
        ;;
    -r|--resources)
        check_resources
        exit $?
        ;;
    "")
        main
        ;;
    *)
        print_error "未知选项: $1"
        show_help
        exit 1
        ;;
esac
