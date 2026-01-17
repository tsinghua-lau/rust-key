#!/bin/bash

# MacOS Key Sound 自动化构建脚本
# MacOS Key Sound Automated Build Script

set -e

PROJECT_NAME="MacOS Key Sound"
DMG_NAME="MacOS-Key-Sound"

echo "🚀 开始构建 MacOS Key Sound 应用..."
echo "🚀 Starting MacOS Key Sound build process..."

# 激活 Rust 环境
echo "🔧 激活 Rust 环境..."
source "$HOME/.cargo/env"

# 清理之前的构建
echo "🧹 清理之前的构建文件..."
cargo clean

# 编译 Release 版本
echo "🔨 编译应用程序..."
cargo build --release

# 检查 cargo-bundle 是否已安装
if ! command -v cargo-bundle &> /dev/null; then
    echo "📦 安装 cargo-bundle..."
    cargo install cargo-bundle
fi

# 创建应用包
echo "📱 创建 macOS 应用包..."
cargo bundle --release

# 创建分发目录并清理旧 DMG
mkdir -p dist
echo "🧹 清理旧 DMG 文件..."
find dist -type f -name "${DMG_NAME}*.dmg" -delete 2>/dev/null || true

# 校验资源文件是否存在
if [ ! -f assets/sound.wav ]; then
  echo "❌ 缺少资源文件 assets/sound.wav" >&2
  echo "请确保声音文件存在后再打包" >&2
  exit 1
fi

# 检查 create-dmg 是否已安装
if ! command -v create-dmg &> /dev/null; then
    echo "⚠️  未找到 create-dmg 工具"
    echo "⚠️  请运行以下命令安装: npm install -g create-dmg"
    echo "📦 应用包已创建: target/release/bundle/osx/$PROJECT_NAME.app"
    echo ""
    echo "📝 手动创建 DMG 的命令:"
    echo "create-dmg 'target/release/bundle/osx/$PROJECT_NAME.app' --overwrite --dmg-title='$PROJECT_NAME' dist/"
    exit 0
fi

# 添加macOS权限配置
echo "🔐 添加macOS权限配置..."
INFO_PLIST="target/release/bundle/osx/$PROJECT_NAME.app/Contents/Info.plist"

if [ -f "$INFO_PLIST" ]; then
  # 检查是否已经包含权限配置
  if ! grep -q "NSAccessibilityUsageDescription" "$INFO_PLIST"; then
    # 备份原始文件
    cp "$INFO_PLIST" "$INFO_PLIST.backup"

    # 在</dict>前添加权限配置
    if [[ "$OSTYPE" == "darwin"* ]]; then
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
    fi
    echo "✅ 权限配置已添加"
  else
    echo "⚠️  权限配置已存在，跳过"
  fi
else
  echo "❌ Info.plist文件不存在"
fi

# 创建 DMG 安装包
echo "💿 创建 DMG 安装包..."
DMG_PATH="dist/${DMG_NAME}.dmg"
create-dmg \
  --overwrite \
  --no-code-sign \
  --dmg-title="$PROJECT_NAME" \
  "target/release/bundle/osx/$PROJECT_NAME.app" \
  dist/

STATUS=$?
if [ $STATUS -ne 0 ]; then
  echo "❌ create-dmg 失败 (exit $STATUS)" >&2
  exit $STATUS
fi

# 查找实际生成的 DMG 文件名（create-dmg 自动命名）
ACTUAL_DMG=$(find dist -name "*.dmg" -type f | head -1)

echo ""
echo "✅ 构建完成！"
echo "✅ Build completed successfully!"
echo ""
echo "📁 输出文件 / Output files:"
echo "📦 应用包 / App bundle: target/release/bundle/osx/$PROJECT_NAME.app"
if [ -n "$ACTUAL_DMG" ]; then
  echo "💿 DMG 安装包 / DMG installer: $ACTUAL_DMG"
else
  echo "💿 DMG 安装包 / DMG installer: 未找到生成的 DMG 文件"
fi
echo ""
echo "📊 文件大小 / File sizes:"
ls -lh "target/release/bundle/osx/$PROJECT_NAME.app/Contents/MacOS/"*
if [ -n "$ACTUAL_DMG" ]; then
  ls -lh "$ACTUAL_DMG"
else
  echo "DMG 文件未创建"
fi
echo ""
echo "🎯 下一步 / Next steps:"
echo "1. 测试 DMG 安装包 / Test the DMG installer"
echo "2. 双击 DMG 文件进行安装测试 / Double-click DMG to test installation"
echo "3. 运行应用时记得授予辅助功能权限 / Grant Accessibility permissions when running"
echo ""
echo "⚠️  重要提醒 / Important reminder:"
echo "   首次运行需要在系统偏好设置中授予辅助功能权限"
echo "   First run requires granting Accessibility permissions in System Preferences"
