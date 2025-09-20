#!/bin/bash

# macOS应用程序签名脚本
# 用法: ./sign_app.sh

set -e

echo "🔧 开始构建和签名macOS应用程序..."

# 1. 构建release版本
echo "📦 构建release版本..."
cargo build --release

# 2. 检查是否存在Apple开发者证书
echo "🔍 检查代码签名证书..."
DEVELOPER_CERT=$(security find-identity -v -p codesigning | grep "Developer ID Application" | head -1 | cut -d '"' -f 2)

if [ -z "$DEVELOPER_CERT" ]; then
    echo "⚠️  未找到Apple开发者证书，将使用ad-hoc签名"
    SIGN_IDENTITY="-"
    NOTARIZE=false
else
    echo "✅ 找到开发者证书: $DEVELOPER_CERT"
    SIGN_IDENTITY="$DEVELOPER_CERT"
    NOTARIZE=true
fi

# 3. 签名应用程序
APP_PATH="target/release/macos-key-sound"
echo "✍️  签名应用程序: $APP_PATH"

codesign --force \
    --options runtime \
    --entitlements entitlements.plist \
    --sign "$SIGN_IDENTITY" \
    "$APP_PATH"

# 4. 验证签名
echo "🔍 验证签名..."
codesign -v "$APP_PATH"

echo "📋 签名详细信息:"
codesign -dv --entitlements - "$APP_PATH"

# 5. 创建.app包（如果需要）
APP_BUNDLE="target/release/MacOS Key Sound.app"
if [ ! -d "$APP_BUNDLE" ]; then
    echo "📱 创建.app包..."
    mkdir -p "$APP_BUNDLE/Contents/MacOS"
    mkdir -p "$APP_BUNDLE/Contents/Resources"
    
    # 复制可执行文件
    cp "$APP_PATH" "$APP_BUNDLE/Contents/MacOS/"
    
    # 复制资源文件
    if [ -d "assets" ]; then
        cp -r assets/* "$APP_BUNDLE/Contents/Resources/"
    fi
    
    # 创建Info.plist
    cat > "$APP_BUNDLE/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>MacOS Key Sound</string>
    <key>CFBundleDisplayName</key>
    <string>MacOS Key Sound</string>
    <key>CFBundleIdentifier</key>
    <string>com.example.macos-key-sound</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>macos-key-sound</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
    <key>LSUIElement</key>
    <true/>
    <key>NSMicrophoneUsageDescription</key>
    <string>此应用需要音频输入权限以播放按键音效</string>
    <key>NSAppleEventsUsageDescription</key>
    <string>此应用需要辅助功能权限以监听键盘事件</string>
</dict>
</plist>
EOF
    
    # 签名.app包
    echo "✍️  签名.app包..."
    codesign --force \
        --options runtime \
        --entitlements entitlements.plist \
        --sign "$SIGN_IDENTITY" \
        --deep \
        "$APP_BUNDLE"
fi

# 6. 公证（如果有开发者证书）
if [ "$NOTARIZE" = true ]; then
    echo "📋 要进行公证，请运行以下命令:"
    echo "xcrun notarytool submit '$APP_BUNDLE' --keychain-profile 'notarytool-password' --wait"
    echo "xcrun stapler staple '$APP_BUNDLE'"
fi

# 7. 测试运行
echo "🧪 测试签名状态..."
if spctl -a -t exec -v "$APP_PATH" 2>/dev/null; then
    echo "✅ 应用程序通过Gatekeeper验证"
else
    echo "⚠️  应用程序未通过Gatekeeper验证（ad-hoc签名为正常现象）"
fi

echo ""
echo "🎉 签名完成！"
echo "📁 应用程序位置: $APP_PATH"
if [ -d "$APP_BUNDLE" ]; then
    echo "📱 .app包位置: $APP_BUNDLE"
fi
echo ""
echo "💡 使用说明:"
echo "1. 运行应用程序前，请确保已授予必要的系统权限"
echo "2. 系统偏好设置 > 安全性与隐私 > 辅助功能"
echo "3. 添加此应用程序到允许列表中"