# DMG 打包详细教程

## 📋 概述

本教程详细介绍如何将 MacOS Key Sound 应用打包成专业的 DMG 安装包，包括所有必要的步骤和最佳实践。

## 🛠 准备工作

### 1. 安装必要工具

```bash
# 1. 确保 Rust 环境正确配置
source "$HOME/.cargo/env"

# 2. 安装 cargo-bundle
cargo install cargo-bundle

# 3. 安装 Node.js（如果尚未安装）
# 访问 https://nodejs.org 下载并安装

# 4. 安装 create-dmg
npm install -g create-dmg

# 5. 验证工具安装
cargo bundle --version
create-dmg --version
```

### 2. 验证项目状态

```bash
# 检查项目文件完整性
ls -la src/main.rs assets/sound.wav Cargo.toml

# 确保音效文件存在
file assets/sound.wav
```

## 🔨 详细打包步骤

### 步骤 1：清理和编译

```bash
# 清理之前的构建产物
cargo clean

# 编译 Release 版本（优化编译）
cargo build --release

# 验证编译结果
ls -la target/release/macos-key-sound
```

### 步骤 2：创建 macOS 应用包

```bash
# 使用 cargo-bundle 创建 .app 包
cargo bundle --release

# 验证应用包创建成功
ls -la "target/release/bundle/osx/MacOS Key Sound.app"

# 检查应用包内容结构
tree "target/release/bundle/osx/MacOS Key Sound.app" || \
find "target/release/bundle/osx/MacOS Key Sound.app" -type f
```

### 步骤 3：准备 DMG 创建环境

```bash
# 创建分发目录
mkdir -p dist

# 清理可能存在的旧 DMG 文件
rm -f "dist/MacOS Key Sound.dmg"

# 验证 .app 文件可执行性
codesign -dv "target/release/bundle/osx/MacOS Key Sound.app" 2>/dev/null || \
echo "应用未签名（这对本地分发是正常的）"
```

### 步骤 4：创建基础 DMG

```bash
# 基础 DMG 创建命令
create-dmg \
  --volname "MacOS Key Sound" \
  --volicon "target/release/bundle/osx/MacOS Key Sound.app/Contents/Resources/icon.icns" \
  --window-pos 200 120 \
  --window-size 600 400 \
  --icon-size 100 \
  --icon "MacOS Key Sound.app" 175 190 \
  --hide-extension "MacOS Key Sound.app" \
  --app-drop-link 425 190 \
  --overwrite \
  "dist/MacOS Key Sound.dmg" \
  "target/release/bundle/osx/MacOS Key Sound.app"
```

**注意**：如果上述命令因为图标文件不存在而失败，使用简化版本：

```bash
# 简化的 DMG 创建命令
create-dmg \
  --volname "MacOS Key Sound" \
  --window-pos 200 120 \
  --window-size 600 400 \
  --icon-size 100 \
  --app-drop-link 425 190 \
  --overwrite \
  "dist/MacOS Key Sound.dmg" \
  "target/release/bundle/osx/MacOS Key Sound.app"
```

### 步骤 5：验证 DMG 包

```bash
# 检查 DMG 文件
ls -lh "dist/MacOS Key Sound.dmg"

# 挂载 DMG 验证内容
hdiutil attach "dist/MacOS Key Sound.dmg"

# 查看挂载内容
ls -la "/Volumes/MacOS Key Sound/"

# 卸载 DMG
hdiutil detach "/Volumes/MacOS Key Sound"
```

## 🎨 高级 DMG 自定义

### 创建自定义背景图片

```bash
# 1. 创建背景图片目录
mkdir -p dmg-assets

# 2. 准备背景图片（推荐尺寸 600x400）
# 将您的背景图片保存为 dmg-assets/background.png

# 3. 使用自定义背景的 DMG 创建命令
create-dmg \
  --volname "MacOS Key Sound" \
  --background "dmg-assets/background.png" \
  --window-pos 200 120 \
  --window-size 600 400 \
  --icon-size 100 \
  --icon "MacOS Key Sound.app" 175 190 \
  --hide-extension "MacOS Key Sound.app" \
  --app-drop-link 425 190 \
  --overwrite \
  "dist/MacOS Key Sound.dmg" \
  "target/release/bundle/osx/MacOS Key Sound.app"
```

### 添加许可协议

```bash
# 1. 创建许可协议文件
cat > LICENSE.txt << 'EOF'
MacOS Key Sound 软件许可协议

本软件仅供学习和个人使用。

使用本软件即表示您同意以下条款：
1. 本软件按"原样"提供，不提供任何形式的保证
2. 作者不对因使用本软件而造成的任何损失承担责任
3. 禁止将本软件用于商业用途

版权所有 © 2024
EOF

# 2. 在 DMG 创建时包含许可协议
create-dmg \
  --volname "MacOS Key Sound" \
  --eula "LICENSE.txt" \
  --window-pos 200 120 \
  --window-size 600 400 \
  --icon-size 100 \
  --app-drop-link 425 190 \
  --overwrite \
  "dist/MacOS Key Sound.dmg" \
  "target/release/bundle/osx/MacOS Key Sound.app"
```

## 🔐 代码签名和公证（可选）

### 开发者证书签名

如果您有 Apple 开发者账号，可以对应用进行签名：

```bash
# 1. 查看可用的签名证书
security find-identity -v -p codesigning

# 2. 对应用进行签名
codesign --force --deep --sign "Developer ID Application: Your Name (TEAM_ID)" \
  "target/release/bundle/osx/MacOS Key Sound.app"

# 3. 验证签名
codesign -dv "target/release/bundle/osx/MacOS Key Sound.app"
spctl -a -v "target/release/bundle/osx/MacOS Key Sound.app"
```

### 公证流程

```bash
# 1. 创建签名的 DMG
create-dmg \
  --volname "MacOS Key Sound" \
  --window-pos 200 120 \
  --window-size 600 400 \
  --icon-size 100 \
  --app-drop-link 425 190 \
  --overwrite \
  "dist/MacOS Key Sound.dmg" \
  "target/release/bundle/osx/MacOS Key Sound.app"

# 2. 对 DMG 进行签名
codesign --force --sign "Developer ID Application: Your Name (TEAM_ID)" \
  "dist/MacOS Key Sound.dmg"

# 3. 提交公证（需要 App Store Connect API 密钥）
xcrun notarytool submit "dist/MacOS Key Sound.dmg" \
  --key-id "YOUR_KEY_ID" \
  --key "AuthKey_YOUR_KEY_ID.p8" \
  --issuer "YOUR_ISSUER_ID" \
  --wait

# 4. 装订公证票据
xcrun stapler staple "dist/MacOS Key Sound.dmg"
```

## 🧪 测试和验证

### 本地测试

```bash
# 1. 挂载 DMG
open "dist/MacOS Key Sound.dmg"

# 2. 手动拖拽安装到应用程序文件夹

# 3. 从启动台运行应用

# 4. 测试权限授予流程

# 5. 测试按键音效功能
```

### 自动化验证脚本

```bash
#!/bin/bash
# dmg-test.sh - DMG 验证脚本

set -e

DMG_PATH="dist/MacOS Key Sound.dmg"
APP_NAME="MacOS Key Sound"

echo "🧪 开始 DMG 验证..."

# 检查 DMG 文件存在
if [ ! -f "$DMG_PATH" ]; then
    echo "❌ DMG 文件不存在: $DMG_PATH"
    exit 1
fi

# 获取 DMG 信息
echo "📊 DMG 信息:"
ls -lh "$DMG_PATH"

# 挂载 DMG
echo "💿 挂载 DMG..."
MOUNT_POINT=$(hdiutil attach "$DMG_PATH" | grep "Volumes" | cut -f3)

if [ -z "$MOUNT_POINT" ]; then
    echo "❌ DMG 挂载失败"
    exit 1
fi

echo "✅ DMG 已挂载到: $MOUNT_POINT"

# 检查应用包
APP_PATH="$MOUNT_POINT/$APP_NAME.app"
if [ -d "$APP_PATH" ]; then
    echo "✅ 应用包存在: $APP_PATH"

    # 检查应用可执行性
    if [ -x "$APP_PATH/Contents/MacOS/$APP_NAME" ]; then
        echo "✅ 应用可执行文件正常"
    else
        echo "⚠️  应用可执行文件可能有问题"
    fi
else
    echo "❌ 应用包不存在"
fi

# 卸载 DMG
echo "🔄 卸载 DMG..."
hdiutil detach "$MOUNT_POINT"

echo "✅ DMG 验证完成"
```

```bash
# 给脚本执行权限并运行
chmod +x dmg-test.sh
./dmg-test.sh
```

## 📦 完整的自动化打包脚本

创建一个完整的打包脚本：

```bash
#!/bin/bash
# comprehensive-build.sh - 完整打包脚本

set -e

PROJECT_NAME="MacOS Key Sound"
VERSION="1.0.0"
DIST_DIR="dist"

echo "🚀 开始完整打包流程..."

# 1. 环境检查
echo "🔍 检查环境..."
source "$HOME/.cargo/env"

if ! command -v cargo-bundle &> /dev/null; then
    echo "📦 安装 cargo-bundle..."
    cargo install cargo-bundle
fi

if ! command -v create-dmg &> /dev/null; then
    echo "⚠️  create-dmg 未安装，请运行: npm install -g create-dmg"
    exit 1
fi

# 2. 清理和编译
echo "🧹 清理项目..."
cargo clean

echo "🔨 编译项目..."
cargo build --release

# 3. 创建应用包
echo "📱 创建应用包..."
cargo bundle --release

# 4. 准备分发目录
echo "📁 准备分发目录..."
mkdir -p "$DIST_DIR"
rm -f "$DIST_DIR/$PROJECT_NAME.dmg"

# 5. 创建 DMG
echo "💿 创建 DMG 安装包..."
create-dmg \
  --volname "$PROJECT_NAME" \
  --window-pos 200 120 \
  --window-size 600 400 \
  --icon-size 100 \
  --app-drop-link 425 190 \
  --overwrite \
  "$DIST_DIR/$PROJECT_NAME.dmg" \
  "target/release/bundle/osx/$PROJECT_NAME.app"

# 6. 验证结果
echo "✅ 打包完成！"
echo ""
echo "📦 应用包: target/release/bundle/osx/$PROJECT_NAME.app"
echo "💿 DMG 安装包: $DIST_DIR/$PROJECT_NAME.dmg"
echo ""
echo "📊 文件信息:"
ls -lh "$DIST_DIR/$PROJECT_NAME.dmg"
echo ""
echo "🎯 下一步:"
echo "1. 测试 DMG 安装包"
echo "2. 分发给用户"
echo "3. 提醒用户授予辅助功能权限"
```

## 🚀 快速打包命令总结

如果您只需要快速创建 DMG，使用以下一键命令：

```bash
# 一键打包命令
source "$HOME/.cargo/env" && \
cargo clean && \
cargo build --release && \
cargo bundle --release && \
mkdir -p dist && \
create-dmg \
  --volname "MacOS Key Sound" \
  --window-pos 200 120 \
  --window-size 600 400 \
  --icon-size 100 \
  --app-drop-link 425 190 \
  --overwrite \
  "dist/MacOS Key Sound.dmg" \
  "target/release/bundle/osx/MacOS Key Sound.app" && \
echo "✅ DMG 创建完成: dist/MacOS Key Sound.dmg"
```

## 📋 最终检查清单

在分发 DMG 之前，请确认：

- [ ] DMG 文件能正常挂载
- [ ] 应用可以正确拖拽到应用程序文件夹
- [ ] 应用能在不同 macOS 版本上运行
- [ ] 权限授予流程正常工作
- [ ] 音效文件能正确播放
- [ ] 键盘监听功能正常
- [ ] 应用退出机制正常

---

完成以上步骤后，您就有了一个专业的 macOS DMG 安装包，可以分发给用户使用！
