"""从已构建的 Tauri app 生成带固定 Finder 布局的 macOS 安装盘。"""

import argparse
import json
import platform
import plistlib
import subprocess
import tempfile
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=("aarch64-apple-darwin", "x86_64-apple-darwin"))
    args = parser.parse_args()
    if platform.system() != "Darwin":
        parser.error("DMG 打包只能在 macOS 上运行")

    try:
        import dmgbuild
    except ImportError:
        parser.error("请先在虚拟环境中安装 scripts/dmg-requirements.txt")

    tauri = Path(__file__).resolve().parents[1] / "src-tauri"
    config = json.loads((tauri / "tauri.conf.json").read_text())
    layout = config["bundle"]["macOS"]["dmg"]
    name = config["productName"]
    target_dir = tauri / "target"
    if args.target:
        target_dir /= args.target
    bundle = target_dir / "release" / "bundle"
    app = bundle / "macos" / f"{name}.app"
    if not app.is_dir():
        parser.error(f"未找到 {app}，请先用相同 target 执行 tauri build --bundles app")

    with (app / "Contents" / "Info.plist").open("rb") as stream:
        info = plistlib.load(stream)
    if info["CFBundleShortVersionString"] != config["version"]:
        parser.error("应用版本与 tauri.conf.json 不一致，请重新构建 app")
    arm = args.target == "aarch64-apple-darwin" if args.target else platform.machine() == "arm64"
    architecture = "arm64" if arm else "x86_64"
    executable = app / "Contents" / "MacOS" / info["CFBundleExecutable"]
    subprocess.run(["lipo", str(executable), "-verify_arch", architecture], check=True)

    background = tauri / layout["background"]
    if not background.is_file():
        parser.error(f"未找到安装背景：{background}")
    size = layout["windowSize"]
    position = layout.get("windowPosition", {"x": 200, "y": 200})
    # 不设置 hide_extensions；它会给已签名 app 添加 FinderInfo，破坏严格签名校验。
    settings = {
        "files": [str(app)],
        "symlinks": {"Applications": "/Applications"},
        "icon": str(tauri / "icons" / "icon.icns"),
        "background": str(background),
        "window_rect": ((position["x"], position["y"]), (size["width"], size["height"])),
        "default_view": "icon-view",
        "include_icon_view_settings": True,
        "include_list_view_settings": False,
        "icon_size": 128,
        "text_size": 14,
        # Finder 会拒绝部分 gridSpacing >= 100 的视图配置，显式使用有效值。
        "grid_spacing": 64,
        "arrange_by": None,
        "icon_locations": {
            app.name: (layout["appPosition"]["x"], layout["appPosition"]["y"]),
            "Applications": (layout["applicationFolderPosition"]["x"], layout["applicationFolderPosition"]["y"]),
        },
    }
    suffix = "aarch64" if arm else "x64"
    output = bundle / "dmg" / f"{name}_{config['version']}_{suffix}.dmg"
    output.parent.mkdir(parents=True, exist_ok=True)
    # 构建成功后才替换旧安装包；dmgbuild 负责写入 .DS_Store 与卸载临时映像。
    with tempfile.TemporaryDirectory(prefix=".dmg-", dir=output.parent) as temporary:
        image = Path(temporary) / output.name
        dmgbuild.build_dmg(str(image), name, settings=settings)
        image.replace(output)
    print(f"已生成 {output}")


if __name__ == "__main__":
    main()
