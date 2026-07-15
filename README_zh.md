# TeaQL Forge Rust Server

[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/13621/badge)](https://www.bestpractices.dev/projects/13621)

[🇺🇸 English](README.md) | [🇨🇳 中文](README_zh.md)

TeaQL Forge Server 是完全使用 Rust 编写的 TeaQL 本地代码生成服务。

它将 TeaQL `.xml` 领域模型转换为 Rust 库及完整配置的工程脚手架。它提供了一个基于 `axum` 的高并发后端来处理代码生成、验证模型，并为现代智能 Agent 编程工作流（如 Cursor、Claude 等）提供 AI 辅助代码提示。

## 核心特性
- **纯内存生成:** 直接在内存中渲染 Rust 文件，并作为 `.zip` 文件流式返回，执行速度极快。
- **AI 辅助 API (AI Assist):** 提供动态的 Markdown 模板 (`rust-assist-*`)，包含精确的、感知当前模型的代码片段，以引导 AI 编程助手编写符合 TeaQL 标准的代码（CRUD、查询、列表页等）。
- **实时预览与仪表盘:** 提供交互式 HTML 仪表盘、React Flow 实体关系图以及直接从加载的模型生成的数据字典。
- **多生成目标:** 支持生成纯领域库 (`rust-lib-core`) 或是包含完整 CLI/Server 配置的应用工程脚手架 (`rust-app-console`)。
- **极小多架构 Docker 镜像:** 使用 `cargo-zigbuild` 和 `musl` 基于 `scratch` 构建静态二进制。最终镜像极其轻量（约 5MB），原生支持 `linux/amd64` 和 `linux/arm64`。

## Docker 快速开始

运行本地服务器最简单的方法是通过 Docker。此镜像可完美运行在 Linux、Windows (WSL) 和 Apple Silicon (M1/M2/M3) Mac 上。

```bash
docker run -d --name teaql-forge-server -p 8080:8080 teaql/teaql-forge-rs:latest
```

运行后，打开浏览器访问交互式仪表盘：
👉 **[http://localhost:8080/](http://localhost:8080/)**

### 接入 AI 助手
将你的 CLI 工具或 AI IDE 指向本地服务器，即可解锁实时且感知模型的代码智能提示：
```bash
export TEAQL_ENDPOINT_PREFIX=http://localhost:8080/
```

## 搭配 `cargo-teaql` 使用

你可以无缝地将本地服务器接入你现有的 `cargo-teaql` CLI 工作流中：

```bash
# 生成完整的 Workspace
cargo-teaql gen-workspace --endpoint-prefix http://127.0.0.1:8080/ models/model.xml

# 使用标准生成命令
cargo teaql --input model.xml rust-app-console
```

## 可用的 API 路由

- `GET /` - 带有实时预览功能的交互式 HTML 仪表盘。
- `GET /version` - 返回当前服务器版本。
- `GET /model-view.html` - 交互式实体关系图 (React Flow)。
- `GET /data-design-react.html` - 数据字典 UI。
- `GET /rust-assist-*/[entity]` - 生成特定操作的 AI 上下文提示 (例如 `rust-assist-query/platform`)。
- `POST /generate` - 接收 `multipart/form-data` 请求，包含 `xml` 文件以及 `scope` 目标 (`rust-lib-core` 或 `rust-app-console`)。
- `POST /evaluate` - 评估执行 KSML 脚本和表达式。

## 构建与发布

本服务器使用 `cargo-zigbuild` 实现真正的多架构静态编译。

### 本地开发
```bash
cargo run --bin teaql-forge-server -- --host 127.0.0.1 --port 8080
```

### 构建 Docker 镜像 (发布)
要发布新版本的 Docker 镜像到 Docker Hub，请使用自动构建脚本。这需要在本地安装 `cargo-zigbuild` (`cargo install cargo-zigbuild`)。

```bash
# 以 "latest" 标签发布
./publish_docker.sh

# 以特定版本标签发布
./publish_docker.sh v0.9.0
```

脚本将自动执行：
1. 静态交叉编译 `x86_64` 和 `aarch64` 版本。
2. 将二进制打包为基于 scratch 的极小镜像 (`-amd64` 和 `-arm64`)。
3. 创建并推送统一的 Docker Manifest（多架构清单）。

## 手动配置

在手动运行二进制文件时，你可以传递以下参数：
- `--host`: 绑定的接口 (例如 `0.0.0.0` 暴露给所有网络，`127.0.0.1` 仅限本地访问)。
- `-p, --port`: 监听端口 (默认 `8081`)。

> **警告:** 绑定到 `0.0.0.0` 会将 TeaQL 本地服务器暴露到网络中。在生产环境中请使用企业模式或配置 TLS/认证。

## 许可证
Apache-2.0 License
