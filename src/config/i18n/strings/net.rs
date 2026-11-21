use super::Text;

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

pub const CANNOT_OPEN_A_NETWORK_SESSION: Text = Text {
    zh: "无法初始化网络会话",
    en: "cannot open a network session",
};
