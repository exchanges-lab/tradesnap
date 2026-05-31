# TradeSnap

TradeSnap 是一个轻量级、高性能的 TradingView 图表截图抓取服务，使用 Rust 语言重写自 Python 的 `tradingview-snapshot` 项目。

该项目基于 `headless_chrome` 浏览器自动化库抓取 TradingView 图表，并通过 `Axum` 框架构建高性能的 HTTP API 服务，支持直接获取图表分享直链或 Base64 图片数据。

---

## 1. 核心功能

*   **双模式截图抓取**：
    *   **链接模式**：通过模拟点击 UI 的截图并复制链接，获取 `tradingview.com/x/...` 原始分享链接，并自动将其转换为 TradingView 官方 S3 存储的直链（如 `https://s3.tradingview.com/snapshots/...`）。
    *   **图片数据模式**：通过在页面派发键盘快捷键（`Ctrl+Shift+S`），直接从剪贴板捕获二进制 PNG 图片，并以 Base64 (`data:image/png;base64,...`) 数据 URL 形式返回。
*   **智能时间周期转换**：自动将人性化的时间周期输入（例如 `1h`、`4h`、`1D`、`15m`）转换为 TradingView 认可的标准参数（如 `60`、`240`、`D`）。
*   **安全自动登录**：支持通过注入 `sessionid` 与 `sessionid_sign` 会话 Cookies，自动以 TradingView 登录用户身份访问页面，绕过未登录游客的限制。
*   **异步线程隔离**：基于 Tokio 运行时开发，使用 `tokio::task::spawn_blocking` 将阻塞的 Chromium 实例隔离运行，确保高并发 HTTP API 服务吞吐率。
*   **零侵入 Clipboard 权限配置**：每次启动时自动配置 Chrome Profile 的用户 Preferences 首选项，允许指定域名无感知读写剪贴板，彻底避免 `NotAllowedError: Write permission denied` 错误。

---

## 2. 目录结构

```text
tradesnap/
├── src/
│   ├── lib.rs          # 统一导出公共模块及核心服务
│   ├── main.rs         # 启动 Axum HTTP API 服务
│   ├── scraper.rs      # TradingView 浏览器自动化核心抓取器
│   ├── config.rs       # 环境变量与配置文件载入器
│   └── structs.rs      # 公共数据结构定义
├── examples/           # 示例目录
│   └── simple.rs       # 浏览器交互式独立调试脚本
├── tests/              # 测试目录
│   └── config_test.rs  # 配置与转换正则集成测试
├── .env                # 本地运行环境密钥文件（已忽略提交）
├── .env.example        # 环境变量模板
├── Cargo.toml          # Rust 包依赖与定义文件
└── README.md           # 本说明文档
```

---

## 3. 环境变量配置 (`.env`)

在项目根目录下创建 `.env` 文件，并填写如下配置：

| 环境变量名 | 用途 | 是否必填 | 默认值 | 示例值 |
| :--- | :--- | :--- | :--- | :--- |
| `RUST_LOG` | 日志级别过滤 | 否 | `info` | `debug` |
| `TRADINGVIEW_SESSION_ID` | TradingView 的 `sessionid` Cookie | 是 | 无 | 您的登录 Cookie 字符串 |
| `TRADINGVIEW_SESSION_ID_SIGN` | TradingView 的 `sessionid_sign` Cookie | 是 | 无 | 您的登录签名 Cookie 字符串 |
| `MCP_SCRAPER_HEADLESS` | 是否无头（后台）运行浏览器 | 否 | `true` | `true`/`false` |
| `MCP_SCRAPER_WINDOW_WIDTH` | 浏览器视口宽度 | 否 | `1920` | `1920` |
| `MCP_SCRAPER_WINDOW_HEIGHT` | 浏览器视口高度 | 否 | `1080` | `1080` |
| `MCP_SCRAPER_CHART_PAGE_ID` | 用户自定义 TradingView 图表保存布局 ID | 否 | - | `您的私有布局ID` |
| `MCP_SCRAPER_USE_SAVE_SHORTCUT` | `true` 返回 Base64 图片；`false` 返回 S3 直链 | 否 | `true` | `false` |

---

## 4. 安装与运行

### 4.1 准备工作
确保系统已安装 Chromium 浏览器。如果在 Linux/Ubuntu 环境中运行，程序会自动寻找 Snap 安装的 Chromium 地址 `/snap/bin/chromium`。

### 4.2 编译与启动

```bash
# 1. 复制并编辑配置文件
cp .env.example .env

# 2. 启动服务 (默认监听端口 8003)
cargo run
```

---

## 5. API 接口文档

### 5.1 获取图表快照 `GET /chart`

**请求参数：**
*   `ticker` (String): TradingView 标的符号（如 `BYBIT:BTCUSDT.P`、`NASDAQ:AAPL`）
*   `interval` (String): 时间周期（支持 `1h`、`4h`、`1d`、`15m` 等格式）

**测试请求：**
```bash
curl "http://localhost:8003/chart?ticker=BYBIT:BTCUSDT.P&interval=4h"
```

**响应格式 (链接模式 - `MCP_SCRAPER_USE_SAVE_SHORTCUT="false"`)：**
```json
{
  "ticker": "BYBIT:BTCUSDT.P",
  "interval": "4h",
  "image_url": "https://in.tradingview.com/x/abCdEfG/",
  "png_url": "https://s3.tradingview.com/snapshots/a/abCdEfG.png"
}
```

**响应格式 (图片模式 - `MCP_SCRAPER_USE_SAVE_SHORTCUT="true"`)：**
```json
{
  "ticker": "BYBIT:BTCUSDT.P",
  "interval": "4h",
  "image_url": "data:image/png;base64,iVBORw0KGgo...",
  "png_url": "data:image/png;base64,iVBORw0KGgo..."
}
```

### 5.2 健康检查 `GET /`

**请求：**
```bash
curl "http://localhost:8003/"
```

**响应：**
```json
{"status": "ok"}
```
