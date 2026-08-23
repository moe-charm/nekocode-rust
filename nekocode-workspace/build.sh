#!/bin/bash
# 🚀 NekoCode ビルド＆自動リリース更新スクリプト
# これを使えば必ずreleasesフォルダが最新になる！

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
if [ "${1:-}" = "--rust-first" ]; then
    shift
    exec "$SCRIPT_DIR/../scripts/update_rust_first_release.sh" "$@"
fi

echo "🔨 NekoCode ビルド開始..."
cargo build --release

if [ $? -eq 0 ]; then
    echo "✅ ビルド成功！"
    echo "📦 releasesフォルダを更新中..."
    
    # releasesフォルダに自動コピー
    cp -f target/release/nekocode ../../releases/
    cp -f target/release/nekorefactor ../../releases/
    cp -f target/release/nekoimpact ../../releases/
    cp -f target/release/nekoinc ../../releases/
    cp -f target/release/nekomcp ../../releases/
    
    echo "✨ 完了！releasesフォルダが最新になりました！"
    
    # バージョン確認
    echo "📋 確認："
    ../../releases/nekocode --version
else
    echo "❌ ビルド失敗"
    exit 1
fi
