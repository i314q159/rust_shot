#[derive(Clone, Copy, Debug)]
pub struct Text {
    pub zh: &'static str,
    pub en: &'static str,
}

pub const RUNNING_IN_THE_TRAY_F1_REGION_F2_FULL_SCREEN: Text = Text {
    zh: "已在托盘常驻：F1 截图 · F2 全屏",
    en: "Running in the tray: F1 region · F2 full screen",
};
pub const HIDDEN_TO_TRAY_F1_CAPTURES_RIGHT_CLICK_THE_TRAY: Text = Text {
    zh: "已隐藏到托盘：F1 截图 · 右键托盘图标可退出",
    en: "Hidden to tray: F1 captures · right-click the tray icon to quit",
};
pub const PNG_IMAGE: Text = Text {
    zh: "PNG 图片",
    en: "PNG image",
};
pub const CHOOSING_WHERE_TO_SAVE: Text = Text {
    zh: "正在选择保存位置…",
    en: "Choosing where to save…",
};
pub const THE_SAVE_DIALOG_IS_ALREADY_OPEN: Text = Text {
    zh: "保存对话框已经打开了",
    en: "The save dialog is already open",
};
pub const AUTO_SAVE_FAILED: Text = Text {
    zh: "，自动保存失败：{}",
    en: ", auto-save failed: {}",
};
pub const SAVED: Text = Text {
    zh: "，已保存 {}",
    en: ", saved {}",
};
pub const COPY_FAILED: Text = Text {
    zh: "复制失败：{}",
    en: "Copy failed: {}",
};
pub const COPIED_TO_CLIPBOARD: Text = Text {
    zh: "已复制到剪贴板",
    en: "Copied to clipboard",
};
pub const CANCELLED: Text = Text {
    zh: "已取消",
    en: "Cancelled",
};
pub const COULD_NOT_CREATE_THE_CAPTURE_OVERLAY_TRY_AGAIN: Text = Text {
    zh: "创建截图覆盖层失败，请重试",
    en: "Could not create the capture overlay, try again",
};
pub const CAPTURE_FAILED_CANNOT_READ_THE_SCREEN: Text = Text {
    zh: "截屏失败：无法读取屏幕内容",
    en: "Capture failed: cannot read the screen",
};
pub const SAVE_FAILED_THE_DIALOG_THREAD_EXITED: Text = Text {
    zh: "保存失败：对话框线程退出",
    en: "Save failed: the dialog thread exited",
};
pub const SAVE_CANCELLED: Text = Text {
    zh: "已取消保存",
    en: "Save cancelled",
};
pub const SAVE_FAILED: Text = Text {
    zh: "保存失败：{}",
    en: "Save failed: {}",
};
pub const SAVED_2: Text = Text {
    zh: "已保存 {}",
    en: "Saved {}",
};
pub const FOLDER_SELECTION_FAILED: Text = Text {
    zh: "目录选择失败",
    en: "Folder selection failed",
};
pub const FOLDER_SELECTION_CANCELLED: Text = Text {
    zh: "已取消选择目录",
    en: "Folder selection cancelled",
};
pub const SAVE_FOLDER: Text = Text {
    zh: "保存目录：{}",
    en: "Save folder: {}",
};
pub const CHOOSING_A_SAVE_FOLDER: Text = Text {
    zh: "正在选择保存目录…",
    en: "Choosing a save folder…",
};
pub const THE_FOLDER_PICKER_IS_ALREADY_OPEN: Text = Text {
    zh: "目录选择框已经打开了",
    en: "The folder picker is already open",
};
pub const THE_UPDATE_THREAD_EXITED_UNEXPECTEDLY: Text = Text {
    zh: "检查更新的线程意外退出",
    en: "the update thread exited unexpectedly",
};
pub const UPDATE_CHECK_FAILED: Text = Text {
    zh: "检查更新失败：{}",
    en: "Update check failed: {}",
};
pub const UP_TO_DATE_GITHUB_LATEST_V: Text = Text {
    zh: "已是最新版本（GitHub 最新 v{}）",
    en: "Up to date (GitHub latest v{})",
};
pub const NEW_VERSION_V_IS_AVAILABLE_SEE_THE_UPDATE_WINDOW: Text = Text {
    zh: "发现新版本 v{}，可在更新窗口查看",
    en: "New version v{} is available, see the update window",
};
pub const NEW_VERSION_V_FOUND_UPDATING_SILENTLY: Text = Text {
    zh: "发现新版本 v{}，静默更新中…",
    en: "New version v{} found, updating silently…",
};
pub const THE_UPDATE_THREAD_EXITED_UNEXPECTEDLY_2: Text = Text {
    zh: "更新线程意外退出",
    en: "the update thread exited unexpectedly",
};
pub const UPDATE_FAILED: Text = Text {
    zh: "更新失败：{}",
    en: "Update failed: {}",
};
pub const THE_DOWNLOAD_DOES_NOT_MATCH_SHA256SUMS_TXT: Text = Text {
    zh: "下载的文件与 SHA256SUMS.txt 不一致，已停止安装",
    en: "the download does not match SHA256SUMS.txt, install aborted",
};
pub const DOWNLOADING_THE_UPDATE: Text = Text {
    zh: "正在下载更新…",
    en: "Downloading the update…",
};
pub const PORTABLE_BUILDS_DO_NOT_SELF_UPDATE_DOWNLOAD_THE: Text = Text {
    zh: "便携版不自动更新：请从发布页下载新的便携包",
    en: "Portable builds do not self-update: download the new portable zip from the releases page",
};
pub const PROXY: Text = Text {
    zh: "代理：{}",
    en: "Proxy: {}",
};
pub const INVALID_PROXY_ADDRESS_KEEPING_THE_CURRENT_ONE: Text = Text {
    zh: "代理地址无效，仍然沿用当前设置",
    en: "Invalid proxy address, keeping the current one",
};
pub const LANGUAGE: Text = Text {
    zh: "语言：{}",
    en: "Language: {}",
};
pub const RUSTSHOT: Text = Text {
    zh: "RustShot 截图",
    en: "RustShot",
};
pub const QUIT: Text = Text {
    zh: "退出",
    en: "Quit",
};
pub const SHOW_PANEL: Text = Text {
    zh: "显示主面板",
    en: "Show panel",
};
pub const FULL_SCREEN: Text = Text {
    zh: "全屏截图",
    en: "Full screen",
};
pub const REGION_CAPTURE: Text = Text {
    zh: "区域截图",
    en: "Region capture",
};
pub const THEME: Text = Text {
    zh: "主题：{}",
    en: "Theme: {}",
};
pub const CHECKING_FOR_UPDATES: Text = Text {
    zh: "正在检查更新…",
    en: "Checking for updates…",
};
pub const RUSTSHOT_SCREENSHOT: Text = Text {
    zh: "RustShot 截图工具",
    en: "RustShot Screenshot",
};
pub const CANCEL: Text = Text {
    zh: "取消",
    en: "Cancel",
};
pub const SAVE: Text = Text {
    zh: "保存",
    en: "Save",
};
pub const COPY: Text = Text {
    zh: "复制",
    en: "Copy",
};
pub const UNDO: Text = Text {
    zh: "撤销",
    en: "Undo",
};
pub const L: Text = Text { zh: "粗", en: "L" };
pub const M: Text = Text { zh: "中", en: "M" };
pub const S: Text = Text { zh: "细", en: "S" };
pub const DRAG_A_REGION_CLICK_A_WINDOW_RIGHT_CLICK_OR_ESC: Text = Text {
    zh: "拖动鼠标框选区域 · 单击选中窗口 · 右键或 ESC 取消",
    en: "Drag a region · click a window · right-click or ESC to cancel",
};
pub const NUMBER: Text = Text {
    zh: "序号",
    en: "Number",
};
pub const MOSAIC: Text = Text {
    zh: "马赛克",
    en: "Mosaic",
};
pub const TEXT: Text = Text {
    zh: "文字",
    en: "Text",
};
pub const PEN: Text = Text {
    zh: "画笔",
    en: "Pen",
};
pub const ARROW: Text = Text {
    zh: "箭头",
    en: "Arrow",
};
pub const RECT: Text = Text {
    zh: "矩形",
    en: "Rect",
};
pub const SELECT: Text = Text {
    zh: "选择",
    en: "Select",
};

#[cfg(test)]
pub const TEST_PAIR: Text = Text {
    zh: "中文",
    en: "English",
};
pub const FOLLOW_SYSTEM: Text = Text {
    zh: "跟随系统",
    en: "Follow system",
};
pub const CONFIG_HEADER: Text = Text {
    zh: "# RustShot 配置（保存后重启生效；save_dir 支持 %USERPROFILE% 这类环境变量）\n\
             # theme: system | light | dark   lang: system | zh | en\n\
             # auto_update: notice | silent   proxy: 留空跟随系统 | direct | 7890 | host:port\n",
    en: "# RustShot config (restart to apply; save_dir accepts %USERPROFILE% variables)\n\
             # theme: system | light | dark   lang: system | zh | en\n\
             # auto_update: notice | silent   proxy: empty follows Windows | direct | 7890 | host:port\n",
};
pub const PROXY_2: Text = Text {
    zh: "（当前代理：{}）",
    en: " (proxy: {})",
};
pub const CANNOT_OPEN_A_NETWORK_SESSION: Text = Text {
    zh: "无法初始化网络会话",
    en: "cannot open a network session",
};
pub const DIRECT_NO_PROXY: Text = Text {
    zh: "直连（不使用代理）",
    en: "direct (no proxy)",
};
pub const SYSTEM_PROXY: Text = Text {
    zh: "系统代理",
    en: "system proxy",
};
pub const DARK: Text = Text {
    zh: "深色",
    en: "Dark",
};
pub const LIGHT: Text = Text {
    zh: "浅色",
    en: "Light",
};
pub const CLOSE: Text = Text {
    zh: "关闭",
    en: "Close",
};
pub const OPEN_RELEASES_PAGE: Text = Text {
    zh: "打开发布页",
    en: "Open releases page",
};
pub const CHECK_AGAIN: Text = Text {
    zh: "重新检查",
    en: "Check again",
};
pub const UPDATE_NOW: Text = Text {
    zh: "立即更新",
    en: "Update now",
};
pub const RETRY_UPDATE: Text = Text {
    zh: "重试更新",
    en: "Retry update",
};
pub const PORTABLE_BUILDS_DO_NOT_SELF_UPDATE_DOWNLOAD_THE_2: Text = Text {
    zh: "便携版不自动更新，请下载新的便携包",
    en: "Portable builds do not self-update; download the new portable zip",
};
pub const UPDATE_FAILED_2: Text = Text {
    zh: "更新失败",
    en: "Update failed",
};
pub const VERIFIED_INSTALLING_AND_RESTARTING: Text = Text {
    zh: "校验通过，正在安装并重启…",
    en: "Verified — installing and restarting…",
};
pub const VERIFYING_SHA_256: Text = Text {
    zh: "正在校验 SHA-256…",
    en: "Verifying SHA-256…",
};
pub const DOWNLOADING: Text = Text {
    zh: "正在下载 {}",
    en: "Downloading {}",
};
pub const DOWNLOADING_2: Text = Text {
    zh: "正在下载 {}%",
    en: "Downloading {}%",
};
pub const DOWNLOAD_VERIFY_AND_INSTALL_SILENTLY_ONE_UAC: Text = Text {
    zh: "下载并校验后静默安装，只需要确认一次 UAC，装完自动重启。",
    en: "Download, verify and install silently — one UAC prompt, then a restart.",
};
pub const YOU_CAN_ALSO_OPEN_THE_RELEASES_PAGE: Text = Text {
    zh: "也可以直接打开发布页确认最新版本。",
    en: "You can also open the releases page.",
};
pub const UPDATE_CHECK_FAILED_2: Text = Text {
    zh: "检查更新失败",
    en: "Update check failed",
};
pub const THIS_RELEASE_HAS_NO_NOTES: Text = Text {
    zh: "该版本没有填写更新说明。",
    en: "This release has no notes.",
};
pub const RELEASED: Text = Text {
    zh: "发布于 ",
    en: "released ",
};
pub const NEW_VERSION_V: Text = Text {
    zh: "发现新版本 v{}",
    en: "New version v{}",
};
pub const NEW_RELEASES_CAN_BE_OPENED_FROM_HERE: Text = Text {
    zh: "有新版本发布时可以在这里直接打开发布页下载。",
    en: "New releases can be opened from here.",
};
pub const CONNECTING_TO_GITHUB: Text = Text {
    zh: "正在连接 GitHub 检查最新版本…",
    en: "Connecting to GitHub…",
};
pub const NOT_CHECKED_YET: Text = Text {
    zh: "尚未检查更新",
    en: "Not checked yet",
};
pub const WRITES_AUTO_UPDATE_SILENT_TO_CONFIG_TOML_ONE_UAC: Text = Text {
    zh: "写入 config.toml 的 auto_update = 'silent'，仍会弹一次 UAC",
    en: "writes auto_update = 'silent' to config.toml; one UAC prompt still applies",
};
pub const INSTALL_NEW_VERSIONS_SILENTLY_DO_NOT_ASK: Text = Text {
    zh: "发现新版本后直接静默安装（不再询问）",
    en: "Install new versions silently (do not ask)",
};
pub const CHECK_FOR_UPDATES: Text = Text {
    zh: "检查更新",
    en: "Check for updates",
};
pub const FULL_TEXT_LICENSE_MIT_MD: Text = Text {
    zh: "协议全文：LICENSE-MIT.md",
    en: "Full text: LICENSE-MIT.md",
};
pub const COPY_FULL_TEXT: Text = Text {
    zh: "复制全文",
    en: "Copy full text",
};
pub const LICENSE_MIT: Text = Text {
    zh: "开源声明 · MIT License",
    en: "License · MIT",
};
pub const NOW: Text = Text {
    zh: "当前：{}",
    en: "Now: {}",
};
pub const INVALID_ADDRESS: Text = Text {
    zh: "地址无效",
    en: "invalid address",
};
pub const TYPE_IT_YOURSELF_ENTER_OR_SAVE_WRITES_CONFIG: Text = Text {
    zh: "填好 IP 和端口，回车或点保存写入 config.toml",
    en: "enter the IP and port, then press Enter or Save to write config.toml",
};
pub const PROXY_IP: Text = Text {
    zh: "IP 地址",
    en: "IP address",
};
pub const PROXY_PORT: Text = Text {
    zh: "端口",
    en: "Port",
};
pub const MANUAL_ADDRESS: Text = Text {
    zh: "手动地址",
    en: "Manual address",
};
pub const DIRECT_NO_PROXY_2: Text = Text {
    zh: "直连（不使用代理）",
    en: "Direct (no proxy)",
};
pub const SYSTEM_PROXY_FOLLOW_WINDOWS: Text = Text {
    zh: "系统代理（跟随 Windows 设置）",
    en: "System proxy (follow Windows)",
};
pub const ONLY_USED_BY_THE_UPDATE_CHECK_AND_DOWNLOAD: Text = Text {
    zh: "只影响检查更新和下载安装包；保存后下一次检查立即生效。",
    en: "Only used by the update check and download; applies to the next check.",
};
pub const PROXY_3: Text = Text {
    zh: "网络代理",
    en: "Proxy",
};
pub const HIDE_TO_TRAY: Text = Text {
    zh: "隐藏到托盘",
    en: "Hide to tray",
};
pub const FONT: Text = Text {
    zh: "字体：{}",
    en: "Font: {}",
};
pub const TRAY_UNAVAILABLE: Text = Text {
    zh: "托盘不可用",
    en: "Tray unavailable",
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
pub const RESET: Text = Text {
    zh: "重置",
    en: "Reset",
};
pub const OPEN: Text = Text {
    zh: "打开",
    en: "Open",
};
pub const CHANGE: Text = Text {
    zh: "更改",
    en: "Change",
};
pub const CONFIG_LIVES_IN_CONFIG_TOML_NEXT_TO_THE_EXE: Text = Text {
    zh: "配置保存在程序目录下的 config.toml",
    en: "Config lives in config.toml next to the exe",
};
pub const CONFIG_EDIT_SAVE_DIR_AUTO_SAVE_THEME_LANG_BY: Text = Text {
    zh: "配置保存在 {}\n可手动编辑 save_dir / auto_save / theme / lang",
    en: "Config: {}\nedit save_dir / auto_save / theme / lang by hand",
};
pub const SAVE_FOLDER_2: Text = Text {
    zh: "保存目录",
    en: "Save folder",
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
pub const LICENSE_MIT_2: Text = Text {
    zh: "开源声明（MIT）",
    en: "License (MIT)",
};
pub const PROXY_NOW: Text = Text {
    zh: "网络代理…（当前：{}）",
    en: "Proxy… (now: {})",
};
pub const OPEN_RELEASES_PAGE_GITHUB: Text = Text {
    zh: "打开发布页（GitHub）",
    en: "Open releases page (GitHub)",
};
pub const HELP: Text = Text {
    zh: "帮助",
    en: "Help",
};
pub const FOLLOW_SYSTEM_NOW: Text = Text {
    zh: "跟随系统（当前{}）",
    en: "Follow system (now {})",
};
pub const LANGUAGE_2: Text = Text {
    zh: "语言",
    en: "Language",
};
pub const THEME_2: Text = Text {
    zh: "主题",
    en: "Theme",
};
pub const OPEN_SAVE_FOLDER: Text = Text {
    zh: "打开保存目录",
    en: "Open save folder",
};
pub const FULL_SCREEN_F2: Text = Text {
    zh: "全屏截图\tF2",
    en: "Full screen\tF2",
};
pub const REGION_CAPTURE_F1: Text = Text {
    zh: "区域截图\tF1",
    en: "Region capture\tF1",
};
pub const CAPTURE: Text = Text {
    zh: "截图",
    en: "Capture",
};
pub const CANNOT_START_THE_UPDATER: Text = Text {
    zh: "无法启动更新程序：{}",
    en: "cannot start the updater: {}",
};
pub const CANNOT_LOCATE_THE_RUNNING_PROGRAM: Text = Text {
    zh: "无法定位当前程序：{}",
    en: "cannot locate the running program: {}",
};
pub const THE_GITHUB_RESPONSE_IS_NOT_UTF_8: Text = Text {
    zh: "GitHub 返回的内容不是 UTF-8",
    en: "the GitHub response is not UTF-8",
};
pub const THE_GITHUB_RESPONSE_IS_TOO_LARGE: Text = Text {
    zh: "GitHub 返回的内容过大",
    en: "the GitHub response is too large",
};
pub const READING_THE_UPDATE_RESPONSE_FAILED: Text = Text {
    zh: "读取更新响应失败",
    en: "reading the update response failed",
};
pub const THE_UPDATE_URL_HAS_NO_HOST: Text = Text {
    zh: "更新地址缺少主机名",
    en: "the update URL has no host",
};
pub const ONLY_HTTPS_URLS_ARE_SUPPORTED: Text = Text {
    zh: "只支持 https 地址",
    en: "only https URLs are supported",
};
pub const SENDING_THE_UPDATE_REQUEST_FAILED: Text = Text {
    zh: "更新请求发送失败",
    en: "sending the update request failed",
};
pub const CANNOT_CREATE_THE_UPDATE_REQUEST: Text = Text {
    zh: "无法创建更新请求",
    en: "cannot create the update request",
};
pub const CANNOT_REACH_GITHUB: Text = Text {
    zh: "无法连接 GitHub",
    en: "cannot reach GitHub",
};
pub const TRUNCATED: Text = Text {
    zh: "\n…（内容过长，已截断）",
    en: "\n… (truncated)",
};
pub const CANNOT_PARSE_VERSION: Text = Text {
    zh: "无法解析版本号 {}",
    en: "cannot parse version {}",
};
pub const THE_PAYLOAD_HAS_NO_TAG_NAME: Text = Text {
    zh: "GitHub 返回的数据缺少 tag_name",
    en: "the payload has no tag_name",
};
pub const NO_VERSION_IN_THE_RELEASES_PAGE_URL: Text = Text {
    zh: "发布页地址里没有版本号",
    en: "no version in the releases page URL",
};
pub const THE_RELEASES_PAGE_RETURNED_HTTP: Text = Text {
    zh: "发布页返回 HTTP {}",
    en: "the releases page returned HTTP {}",
};
pub const THE_GITHUB_API_RETURNED_HTTP: Text = Text {
    zh: "GitHub 接口返回 HTTP {}",
    en: "the GitHub API returned HTTP {}",
};
pub const NO_RELEASES_FOUND: Text = Text {
    zh: "GitHub 接口没有找到发布记录",
    en: "no releases found",
};
pub const GITHUB_RATE_LIMITED_THE_REQUEST: Text = Text {
    zh: "GitHub 接口访问受限（可能被限流）",
    en: "GitHub rate-limited the request",
};
pub const THE_RELEASES_PAGE_FAILED_TOO: Text = Text {
    zh: "{}（发布页也读取失败：{}）",
    en: "{} (the releases page failed too: {})",
};
pub const COULD_NOT_READ_THE_TEST_PAYLOAD: Text = Text {
    zh: "读取测试数据失败：{}",
    en: "could not read the test payload: {}",
};
pub const THE_DOWNLOADED_FILE_IS_EMPTY: Text = Text {
    zh: "下载到的文件是空的",
    en: "the downloaded file is empty",
};
pub const CANNOT_WRITE_THE_DOWNLOAD: Text = Text {
    zh: "写入下载文件失败：{}",
    en: "cannot write the download: {}",
};
pub const READING_THE_DOWNLOAD_RESPONSE_FAILED: Text = Text {
    zh: "读取下载响应失败",
    en: "reading the download response failed",
};
pub const THE_UPDATE_DOWNLOAD_FAILED_HTTP: Text = Text {
    zh: "下载更新包失败（HTTP {}）",
    en: "the update download failed (HTTP {})",
};
pub const SENDING_THE_DOWNLOAD_REQUEST_FAILED: Text = Text {
    zh: "下载请求发送失败",
    en: "sending the download request failed",
};
pub const CANNOT_CREATE_THE_DOWNLOAD_REQUEST: Text = Text {
    zh: "无法创建下载请求",
    en: "cannot create the download request",
};
pub const CANNOT_WRITE_THE_DOWNLOAD_2: Text = Text {
    zh: "无法写入下载文件：{}",
    en: "cannot write the download: {}",
};
pub const CANNOT_CREATE_THE_DOWNLOAD_FOLDER: Text = Text {
    zh: "无法创建下载目录：{}",
    en: "cannot create the download folder: {}",
};
pub const THE_CHECKSUM_LIST_DOES_NOT_MENTION: Text = Text {
    zh: "校验清单里没有 {}",
    en: "the checksum list does not mention {}",
};
pub const COULD_NOT_FETCH_THE_CHECKSUM_LIST_HTTP: Text = Text {
    zh: "下载校验清单失败（HTTP {}）",
    en: "could not fetch the checksum list (HTTP {})",
};
pub const CANNOT_READ_THE_DOWNLOADED_FILE: Text = Text {
    zh: "无法读取下载的文件：{}",
    en: "cannot read the downloaded file: {}",
};
pub const SYSTEM_DEFAULT: Text = Text {
    zh: "系统默认",
    en: "system default",
};
pub const LIGHT_2: Text = Text {
    zh: "浅色",
    en: "light",
};
pub const DARK_2: Text = Text {
    zh: "深色",
    en: "dark",
};

pub const LANGUAGE_ZH: &str = "中文";
pub const LANGUAGE_EN: &str = "English";

pub const HELP_MESSAGE: Text = Text {
    zh: "RustShot\n\n\
         无参数            启动后常驻托盘（不弹主面板；已在运行则静默退出）\n\
         --panel, --show   启动并显示主面板\n\
         --capture, -c     立即区域截图\n\
         --full, -f        立即全屏截图到剪贴板\n\
         --check-update    检查更新（发现新版本时按 auto_update 设置处理）\n\
         --tray, --silent  同“无参数”，开机自启用\n\
         --selftest <png>  离屏渲染一张测试图并退出\n\n\
         全局热键：F1 区域截图 · F2 全屏 · Ctrl+Alt+Q 退出",
    en: "RustShot\n\n\
         (no args)         start in the tray (already running: exit quietly)\n\
         --panel, --show   start with the main panel\n\
         --capture, -c     region capture right away\n\
         --full, -f        full screen to the clipboard (no overlay)\n\
         --check-update    check for updates (honours auto_update)\n\
         --tray, --silent  same as no arguments (used by autostart)\n\
         --selftest <png>  render a test image and exit\n\n\
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

pub const NET_ERROR: Text = Text {
    zh: "{}（error {} / {}）",
    en: "{} (error {} / {})",
};

pub const UPDATE_NET_ERROR: Text = Text {
    zh: "{}（{}）",
    en: "{} ({})",
};

pub const SELFTEST: Text = Text {
    zh: "RustShot 标注测试：文字 / 中文 / 123",
    en: "RustShot annotation test: text / English / 123",
};
