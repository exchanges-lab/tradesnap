# Changelog

本文件记录项目所有值得注意的变更。
格式遵循 Keep a Changelog，版本号遵循 SemVer。

## [Unreleased]

### Added
- 新增 `MCP_SCRAPER_USE_SAVE_SHORTCUT` 环境变量及配置项。允许选择两种截图捕获模式：`true` 时通过快捷键直接提取剪贴板的 Base64 编码图片数据，`false` 时通过模拟点击 UI 生成并转换 S3 官方快照直链。

### Changed
- 清除了 `.env.example` 中 `MCP_SCRAPER_CHART_PAGE_ID` 的硬编码占位符布局 ID。
- 规范并优化了项目整体文件结构与代码排版格式。

### Fixed
- 为截图请求增加可配置的硬超时；超时后终止卡住的 Chromium 并返回 HTTP 504，避免同步 CDP/剪贴板调用永久占用 scraper 锁。
- 截图失败时直接返回 HTTP 错误，不再在同一个请求内自动重建浏览器并重试。
- 将 24 小时浏览器通信空闲超时缩短为 5 分钟，避免失效 CDP 会话长期挂起。
- **链接模式返回旧快照链接**：`get_screenshot_link` 在点击 "Copy link" 前未清空剪贴板，轮询时可能读到上一次请求残留的旧链接并直接返回，导致连续请求（不同 ticker / interval）拿到相同甚至张冠李戴的快照（如同步 BNB 却返回上一轮 ETH 的链接）。现在每次捕获前先清空剪贴板，确保读取到的始终是本次请求生成的链接。
- 自动清理 Chrome Profile 目录下残留的 Chromium 锁定文件（`SingletonLock`、`SingletonSocket` 和 `SingletonCookie`），避免 Docker 容器环境下重启时出现的冷启动连接超时和崩溃问题。

## [0.2.2] - 2026-05-31
### Fixed
- **浏览器空闲超时断连 (Idle Browser Timeout)**：将 `idle_browser_timeout` 从默认 30 秒调整为 24 小时，解决服务启动后约 30 秒浏览器 WebSocket 连接自动断开导致 `Got a timeout while listening for browser events` 错误的问题。
- **日志初始化顺序 (Tracing Init Order)**：将 `.env` 文件加载提前到 tracing 初始化之前，确保 `RUST_LOG` 环境变量被正确读取，修复启动后无任何日志输出的问题。

### Changed
- **Docker Compose 移除端口映射**：服务通过 cycle 内部网络通信，不再需要将 8003 端口映射到宿主机。

## [0.2.1] - 2026-05-31
### Optimized
- **持久化浏览器会话 (Persistent Browser Session)**：将 Scraper 单例作为持久化句柄常驻在 Axum 状态中，避免了每次 API 请求时冷启动 Chromium 进程，提速 2~3 秒。
- **无刷新登录 (Cookie Pre-injection)**：在浏览器初始化时即写入会话 Cookies，省去了先导航再写入再强制刷新 (Reload) 网页的二重耗时步骤，提速 2 秒。
- **自愈式连接管理 (Auto-healing Browser Connection)**：加入了自愈重连机制，若持久化的浏览器后台会话意外中断、闪退或标签页关闭，会自动重新创建实例并自动重试截图。


## [0.2.0] - 2026-05-31
### Added
- 重写并实现 TradingView 浏览器快照抓取模块 `scraper`，基于 `headless_chrome` 控制 Chromium 行为
- 添加 Chrome Preferences 配置写入器，自动授权剪贴板读取，解决无头环境下的剪贴板拒绝异常
- 实现基于 Axum 的 HTTP 服务，支持端口 `8003` 接收 `GET /chart` 的截图请求并返回结构化数据
- 增加 Base64 截图图片数据提取模式：利用 JS 事件向页面派发 `Ctrl+Shift+S` 快捷键并拉取剪贴板 Base64 内容
- 增加周期转换器 `map_interval`：智能兼容映射常用的时间范围参数，如 `1h` -> `60`、`4h` -> `240`、`1d` -> `D`
- 补充 integration test 单元测试：对时间映射及直链 URL 转换正则表达式进行了全面测试覆盖
- 优化系统多线程架构：在主服务层使用 `spawn_blocking` 运行同步浏览器实例，解决高吞吐异步防卡死问题

## [0.1.0] - 2026-05-31
### Added
- 初始化 Rust 项目结构
- 添加 `config` 配置模块以支持通过 `dotenvy` 加载环境变量
- 集成 `tokio` 异步运行时
- 集成 `thiserror` (库端) 与 `anyhow` (客户端) 错误处理体系
- 集成 `tracing` 与 `tracing-subscriber` 结构化日志规范
- 添加 integration test (`tests/config_test.rs`) 与 示例项目 (`examples/simple.rs`)
