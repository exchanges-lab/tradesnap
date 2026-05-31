# Changelog

本文件记录项目所有值得注意的变更。
格式遵循 Keep a Changelog，版本号遵循 SemVer。

## [Unreleased]
### Added
- (暂无)

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
