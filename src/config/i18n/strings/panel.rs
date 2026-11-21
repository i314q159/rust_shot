use super::Text;

pub const RUNNING_IN_THE_TRAY_F1_REGION_F2_FULL_SCREEN: Text = Text {
    zh: "已在托盘常驻：F1 截图 · F2 全屏",
    en: "Running in the tray: F1 region · F2 full screen",
};

pub const HIDDEN_TO_TRAY_F1_CAPTURES_RIGHT_CLICK_THE_TRAY: Text = Text {
    zh: "已隐藏到托盘：F1 截图 · 右键托盘图标可退出",
    en: "Hidden to tray: F1 captures · right-click the tray icon to quit",
};

pub const FULL_SCREEN: Text = Text {
    zh: "全屏截图",
    en: "Full screen",
};

pub const REGION_CAPTURE: Text = Text {
    zh: "区域截图",
    en: "Region capture",
};

pub const FULL_SCREEN_F2: Text = Text {
    zh: "全屏截图\tF2",
    en: "Full screen\tF2",
};

pub const REGION_CAPTURE_F1: Text = Text {
    zh: "区域截图\tF1",
    en: "Region capture\tF1",
};

pub const THEME: Text = Text {
    zh: "主题：{}",
    en: "Theme: {}",
};

pub const THEME_2: Text = Text {
    zh: "主题",
    en: "Theme",
};

pub const LANGUAGE: Text = Text {
    zh: "语言：{}",
    en: "Language: {}",
};

pub const LANGUAGE_2: Text = Text {
    zh: "语言",
    en: "Language",
};

pub const PROXY: Text = Text {
    zh: "代理：{}",
    en: "Proxy: {}",
};

pub const PROXY_2: Text = Text {
    zh: "（当前代理：{}）",
    en: " (proxy: {})",
};

pub const PROXY_3: Text = Text {
    zh: "网络代理",
    en: "Proxy",
};

pub const PROXY_NOW: Text = Text {
    zh: "网络代理…（当前：{}）",
    en: "Proxy… (now: {})",
};

pub const FONT: Text = Text {
    zh: "字体：{}",
    en: "Font: {}",
};

pub const TRAY_UNAVAILABLE: Text = Text {
    zh: "托盘不可用",
    en: "Tray unavailable",
};

pub const SCREENSHOT_ANNOTATE: Text = Text {
    zh: "截图 · 标注",
    en: "Screenshot · Annotate",
};

pub const CHECKING: Text = Text {
    zh: "检查更新…",
    en: "Checking…",
};

pub const CLICK_TO_SEE_WHAT_S_NEW: Text = Text {
    zh: "点击查看更新内容",
    en: "Click to see what's new",
};

pub const UPDATE_V: Text = Text {
    zh: "可更新 v{}",
    en: "Update v{}",
};

pub const CHECK_FOR_UPDATES: Text = Text {
    zh: "检查更新",
    en: "Check for updates",
};

pub const CHECKING_FOR_UPDATES: Text = Text {
    zh: "正在检查更新…",
    en: "Checking for updates…",
};

pub const LICENSE_MIT: Text = Text {
    zh: "开源声明 · MIT License",
    en: "License · MIT",
};

pub const LICENSE_MIT_2: Text = Text {
    zh: "开源声明（MIT）",
    en: "License (MIT)",
};

pub const OPEN_RELEASES_PAGE: Text = Text {
    zh: "打开发布页",
    en: "Open releases page",
};

pub const OPEN_RELEASES_PAGE_GITHUB: Text = Text {
    zh: "打开发布页（GitHub）",
    en: "Open releases page (GitHub)",
};

pub const OPEN_SAVE_FOLDER: Text = Text {
    zh: "打开保存目录",
    en: "Open save folder",
};

pub const CONFIG_HEADER: Text = Text {
    zh: "# RustShot 配置（保存后重启生效；save_dir 支持 %USERPROFILE% 这类环境变量）\n\
             # theme: system | light | dark   lang: system | zh | en\n\
             # auto_update: notice | silent   proxy: 留空跟随系统 | direct | 7890 | host:port\n",
    en: "# RustShot config (restart to apply; save_dir accepts %USERPROFILE% variables)\n\
             # theme: system | light | dark   lang: system | zh | en\n\
             # auto_update: notice | silent   proxy: empty follows Windows | direct | 7890 | host:port\n",
};

pub const CONFIG_LIVES_IN_CONFIG_TOML_NEXT_TO_THE_EXE: Text = Text {
    zh: "配置保存在程序目录下的 config.toml",
    en: "Config lives in config.toml next to the exe",
};

pub const CONFIG_EDIT_SAVE_DIR_AUTO_SAVE_THEME_LANG_BY: Text = Text {
    zh: "配置保存在 {}\n可手动编辑 save_dir / auto_save / theme / lang",
    en: "Config: {}\nedit save_dir / auto_save / theme / lang by hand",
};

pub const ALSO_SAVE_A_PNG_WHEN_COPYING: Text = Text {
    zh: "复制到剪贴板时同时保存 PNG",
    en: "Also save a PNG when copying",
};

pub const RESET_TO_THE_DEFAULT_FOLDER: Text = Text {
    zh: "已重置为默认目录：{}",
    en: "Reset to the default folder: {}",
};

pub const RESTORE_THE_DEFAULT_FOLDER_AND_WRITE_IT_TO: Text = Text {
    zh: "恢复默认目录并写入配置文件",
    en: "Restore the default folder and write it to config.toml",
};

pub const SAVE_FOLDER: Text = Text {
    zh: "保存目录：{}",
    en: "Save folder: {}",
};

pub const SAVE_FOLDER_2: Text = Text {
    zh: "保存目录",
    en: "Save folder",
};

pub const FOLDER_SELECTION_FAILED: Text = Text {
    zh: "目录选择失败",
    en: "Folder selection failed",
};

pub const FOLDER_SELECTION_CANCELLED: Text = Text {
    zh: "已取消选择目录",
    en: "Folder selection cancelled",
};

pub const CHOOSING_A_SAVE_FOLDER: Text = Text {
    zh: "正在选择保存目录…",
    en: "Choosing a save folder…",
};

pub const THE_FOLDER_PICKER_IS_ALREADY_OPEN: Text = Text {
    zh: "目录选择框已经打开了",
    en: "The folder picker is already open",
};

pub const HELP_MESSAGE: Text = Text {
    zh: "RustShot\n\n\
         无参数            启动后常驻托盘（不弹主面板；已在运行则静默退出）\n\
         --panel, --show   启动并显示主面板\n\
         --capture, -c     立即区域截图\n\
         --full, -f        立即全屏截图到剪贴板\n\
         --check-update    检查更新（发现新版本时按 auto_update 设置处理）\n\
         --tray, --silent  同“无参数”，开机自启用\n\n\
         全局热键：F1 区域截图 · F2 全屏 · Ctrl+Alt+Q 退出",
    en: "RustShot\n\n\
         (no args)         start in the tray (already running: exit quietly)\n\
         --panel, --show   start with the main panel\n\
         --capture, -c     region capture right away\n\
         --full, -f        full screen to the clipboard (no overlay)\n\
         --check-update    check for updates (honours auto_update)\n\
         --tray, --silent  same as no arguments (used by autostart)\n\n\
         Hotkeys: F1 region - F2 full screen - Ctrl+Alt+Q quit",
};

pub const TIPS: [Text; 5] = [
    Text {
        zh: "截图后：拖选区域 · 单击选中窗口",
        en: "After starting: drag a region · click a window",
    },
    Text {
        zh: "标注：矩形 / 箭头 / 画笔 / 文字 / 马赛克 / 序号",
        en: "Tools: rect / arrow / pen / text / mosaic / number",
    },
    Text {
        zh: "复制：Enter / Ctrl+C · 另存：Ctrl+S",
        en: "Copy: Enter / Ctrl+C · Save as: Ctrl+S",
    },
    Text {
        zh: "撤销：Ctrl+Z · 全选：Ctrl+A · 取消：Esc",
        en: "Undo: Ctrl+Z · Select all: Ctrl+A · Cancel: Esc",
    },
    Text {
        zh: "全局热键：F1 区域截图 · F2 全屏（进标注）· Ctrl+Alt+Q 退出",
        en: "Hotkeys: F1 region · F2 full screen (annotate) · Ctrl+Alt+Q quit",
    },
];

pub const LICENSE_CREDIT: Text = Text {
    zh: "RustShot v{} · 作者 {} · MIT License",
    en: "RustShot v{} · by {} · MIT License",
};

pub const UPDATE_CREDIT: Text = Text {
    zh: "当前版本 v{} · 仓库 {}",
    en: "Current v{} · repo {}",
};
