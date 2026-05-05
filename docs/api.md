# Coder API 参考文档

> Coder 是一个 AI 驱动的开发工具，提供 RESTful HTTP API 和 WebSocket 接口，
> 用于管理会话、执行工具和实时通信。

---

## 目录

1. [概述](#1-概述)
2. [基础信息](#2-基础信息)
3. [会话管理 API](#3-会话管理-api)
4. [聊天 API (SSE 流式)](#4-聊天-api-sse-流式)
5. [工具 API](#5-工具-api)
6. [WebSocket API](#6-websocket-api)
7. [健康检查](#7-健康检查)
8. [Library API](#8-library-api)
9. [错误码](#9-错误码)

---

## 1. 概述

Coder API 由 **server** 功能特性提供，基于 Axum 框架构建。

### 启用方式

在 `Cargo.toml` 中启用 `server` 特性：

```toml
[dependencies]
coder = { version = "0.1", features = ["server"] }
```

或通过命令行运行：

```bash
cargo run --features server
```

### 启动服务

```rust
use std::net::SocketAddr;
use std::sync::Arc;
use coder::server::{AppState, serve};

let addr: SocketAddr = "0.0.0.0:3000".parse()?;
let state = Arc::new(AppState::new(
    session_manager,
    tool_registry,
    provider,
));
serve(&addr, state).await?;
```

### 路由总览

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/api/sessions` | 列出所有会话 |
| `POST` | `/api/sessions` | 创建新会话 |
| `GET` | `/api/sessions/{id}` | 获取会话详情 |
| `POST` | `/api/sessions/{id}/chat` | 发送消息（SSE 流式响应） |
| `GET` | `/api/tools` | 列出所有可用工具 |
| `POST` | `/api/tools/{name}/exec` | 执行指定工具 |
| `GET` | `/api/ws` | WebSocket 实时通信 |
| `GET` | `/api/health` | 健康检查 |

---

## 2. 基础信息

### Base URL

```
http://localhost:3000
```

### Content-Type

所有请求和响应均使用 `application/json`，聊天接口除外（使用 `text/event-stream`）。

### 错误响应格式

所有错误响应均遵循统一格式：

```json
{
  "error": "错误描述信息"
}
```

对应的 HTTP 状态码：

| 状态码 | 说明 |
|--------|------|
| `400 Bad Request` | 请求参数错误 |
| `404 Not Found` | 资源不存在 |
| `500 Internal Server Error` | 服务器内部错误 |

---

## 3. 会话管理 API

### 3.1 列出所有会话

```
GET /api/sessions
```

返回所有已保存会话的摘要列表，按更新时间倒序排列。

**响应示例：**

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "title": "New Session",
    "created_at": "2026-05-05T10:00:00+00:00",
    "updated_at": "2026-05-05T10:30:00+00:00",
    "message_count": 5
  }
]
```

**curl 示例：**

```bash
curl http://localhost:3000/api/sessions
```

---

### 3.2 创建新会话

```
POST /api/sessions
```

**请求体（可选）：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `title` | string | 否 | 会话标题，默认值为 `"New Session"` |

**请求示例：**

```json
{
  "title": "My Debug Session"
}
```

**响应示例：**

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "title": "My Debug Session",
  "created_at": "2026-05-05T10:00:00+00:00",
  "updated_at": "2026-05-05T10:00:00+00:00",
  "message_count": 0,
  "messages": []
}
```

**curl 示例：**

```bash
curl -X POST http://localhost:3000/api/sessions \
  -H "Content-Type: application/json" \
  -d '{"title": "My Debug Session"}'

# 使用默认标题
curl -X POST http://localhost:3000/api/sessions
```

---

### 3.3 获取会话详情

```
GET /api/sessions/{id}
```

**路径参数：**

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | string | 会话 UUID |

**响应示例：**

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "title": "My Debug Session",
  "created_at": "2026-05-05T10:00:00+00:00",
  "updated_at": "2026-05-05T10:30:00+00:00",
  "message_count": 2,
  "messages": [
    {
      "role": "user",
      "content": [
        {
          "type": "text",
          "text": "Hello, what can you do?"
        }
      ]
    },
    {
      "role": "assistant",
      "content": [
        {
          "type": "text",
          "text": "I can help you with coding, debugging, and various development tasks."
        }
      ]
    }
  ]
}
```

**curl 示例：**

```bash
curl http://localhost:3000/api/sessions/550e8400-e29b-41d4-a716-446655440000
```

---

## 4. 聊天 API (SSE 流式)

```
POST /api/sessions/{id}/chat
```

向指定会话发送用户消息，并以 **Server-Sent Events (SSE)** 格式流式返回 AI 响应。

### 请求

**路径参数：**

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | string | 会话 UUID |

**请求体：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `message` | string | 是 | 用户消息文本 |

**请求示例：**

```json
{
  "message": "Explain how to use the Axum framework"
}
```

### SSE 响应事件

| 事件名 | 数据格式 | 说明 |
|--------|----------|------|
| `text` | string | 响应文本片段 |
| `done` | JSON 对象 | 流结束，包含停止原因和 token 用量 |
| `error` | string | 错误消息 |

#### text 事件

```
event: text
data: Axum is a web application framework for Rust
```

#### done 事件

```
event: done
data: {"stop_reason":"end_turn","usage":{"input_tokens":42,"output_tokens":156,"total_tokens":198}}
```

`usage` 字段可能为 `null`（当 Provider 不支持用量统计时）。

#### error 事件

```
event: error
data: Provider unavailable: connection refused
```

### 完整示例

```bash
curl -N http://localhost:3000/api/sessions/550e8400-e29b-41d4-a716-446655440000/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "Hello!"}'
```

`-N` 标志禁用 curl 的缓冲，确保实时输出 SSE 事件。

### 处理流程

1. 加载指定 ID 的会话
2. 将用户消息追加到会话消息列表并持久化
3. 向 AI Provider 发起流式请求
4. 通过 SSE 逐块返回 AI 响应
5. 流结束后发送 `done` 事件

### 消息模型

每条消息包含 `role`、`content`、可选的 `name` 和 `tool_call_id`：

```json
{
  "role": "user",
  "content": [
    {
      "type": "text",
      "text": "Hello"
    }
  ]
}
```

**Role 枚举：**

| 值 | 说明 |
|----|------|
| `user` | 用户消息 |
| `assistant` | AI 助手消息 |
| `system` | 系统提示消息 |
| `tool` | 工具调用结果 |

**ContentBlock 枚举：**

| type | 字段 | 说明 |
|------|------|------|
| `text` | `{text: string}` | 文本内容 |
| `tool_use` | `{id, name, input}` | AI 请求调用工具 |
| `tool_result` | `{tool_use_id, content}` | 工具执行结果 |

---

## 5. 工具 API

### 5.1 列出所有可用工具

```
GET /api/tools
```

返回所有已注册工具的列表，包含名称、描述和 JSON Schema 输入定义。

**响应示例：**

```json
[
  {
    "name": "bash",
    "description": "Execute shell commands",
    "schema": {
      "type": "object",
      "properties": {
        "command": {
          "type": "string"
        }
      },
      "required": ["command"]
    }
  },
  {
    "name": "file_read",
    "description": "Read file contents",
    "schema": {
      "type": "object",
      "properties": {
        "file_path": {
          "type": "string"
        }
      },
      "required": ["file_path"]
    }
  }
]
```

**curl 示例：**

```bash
curl http://localhost:3000/api/tools
```

---

### 5.2 执行工具

```
POST /api/tools/{name}/exec
```

**路径参数：**

| 参数 | 类型 | 说明 |
|------|------|------|
| `name` | string | 工具名称，如 `bash`、`file_read`、`grep` 等 |

**请求体：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `args` | JSON 对象 | 是 | 传递给工具的参数，需符合工具定义的 JSON Schema |

**请求示例：**

```json
{
  "args": {
    "command": "echo hello"
  }
}
```

**响应示例：**

```json
{
  "success": true,
  "output": "hello\n",
  "error": null
}
```

失败时的响应：

```json
{
  "success": false,
  "output": "",
  "error": "command not found: foobar"
}
```

**字段说明：**

| 字段 | 类型 | 说明 |
|------|------|------|
| `success` | boolean | 工具是否执行成功 |
| `output` | string | 工具执行的标准输出 |
| `error` | string\|null | 错误消息（成功时为 `null`） |

**curl 示例：**

```bash
curl -X POST http://localhost:3000/api/tools/bash/exec \
  -H "Content-Type: application/json" \
  -d '{"args": {"command": "ls -la"}}'
```

### 可用工具

以下工具在 `tools-core` 特性下默认注册：

| 工具名 | 说明 |
|--------|------|
| `bash` | 执行 Shell 命令 |
| `file_read` | 读取文件内容 |
| `file_write` | 写入文件 |
| `file_edit` | 编辑文件（精确替换） |
| `glob` | 文件通配匹配搜索 |
| `grep` | 文件内容搜索 |
| `question` | 向用户提问 |
| `web_fetch` | 抓取网页内容 |
| `web_search` | 搜索网络 |
| `docs` | 查询文档 |
| `task` | 任务管理 |
| `plan` | 计划管理 |

额外条件编译工具：

| 工具名 | 特性 | 说明 |
|--------|------|------|
| `git` | `tools-git` | Git 操作 |
| `worktree` | `tools-git` | Git Worktree 管理 |
| `docker` | `tools-docker` | Docker 容器管理 |
| `ci` | 默认 | CI 操作 |
| `db_query` | `tools-db` | 数据库查询 |
| `oauth` | `tools-oauth` | OAuth 认证 |

---

## 6. WebSocket API

```
GET /api/ws
```

WebSocket 端点提供全双工实时通信通道，支持聊天、工具执行和心跳检测。

### 连接

```bash
# 使用 wscat
wscat -c ws://localhost:3000/api/ws

# 使用 websocat
websocat ws://localhost:3000/api/ws
```

### 客户端发送消息格式

客户端发送 JSON 消息，格式如下：

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `type` | string | 是 | 消息类型：`"chat"`、`"ping"`、`"tool_exec"` |
| `session_id` | string | 否 | 会话标识符（可选） |
| `payload` | JSON | 否 | 消息负载 |

### 服务器响应事件格式

服务器返回 JSON 事件，格式如下：

| 字段 | 类型 | 说明 |
|------|------|------|
| `type` | string | 事件类型 |
| `data` | JSON\|null | 事件数据 |

### 消息类型

#### ping / pong

客户端发送 `ping`，服务器回复 `pong`，用于连接保活。

**请求：**

```json
{
  "type": "ping",
  "payload": {"timestamp": 1234567890}
}
```

**响应：**

```json
{
  "type": "pong",
  "data": {"timestamp": 1234567890}
}
```

#### tool_exec

客户端请求在服务端执行指定工具。

**请求：**

```json
{
  "type": "tool_exec",
  "payload": {
    "name": "bash",
    "args": {
      "command": "echo hello"
    }
  }
}
```

**响应：**

```json
{
  "type": "tool_result",
  "data": {
    "name": "bash",
    "success": true,
    "output": "hello\n",
    "error": null
  }
}
```

#### chat

> 注意：`chat` 类型在 WebSocket 处理器中已预留但当前版本未实现完整流式。
> 如需完整流式聊天，请使用 SSE 端点 `POST /api/sessions/{id}/chat`。

### 错误响应

```json
{
  "type": "error",
  "data": {
    "message": "invalid message: missing field `type`"
  }
}
```

无效的消息类型：

```json
{
  "type": "error",
  "data": {
    "message": "unknown message type: unknown_type"
  }
}
```

---

## 7. 健康检查

```
GET /api/health
```

无需认证，返回服务器是否正常运行。

**响应示例：**

```
OK
```

**状态码：**

| 状态码 | 说明 |
|--------|------|
| `200 OK` | 服务器正常运行 |

**curl 示例：**

```bash
curl http://localhost:3000/api/health
```

---

## 8. Library API

Coder 也可以作为 Rust 库集成到其他项目中。

### 添加依赖

```toml
[dependencies]
coder = { version = "0.1", features = ["server"] }
```

### 核心类型

#### AppState

共享应用状态，包含会话管理器、工具注册中心和 AI Provider。

```rust
use std::sync::Arc;
use coder::server::AppState;

let state = Arc::new(AppState::new(
    session_manager,        // SessionManager
    tool_registry,          // Arc<ToolRegistry>
    provider,               // Box<dyn Provider>
));
```

#### 启动服务

```rust
use std::net::SocketAddr;
use coder::server::serve;

let addr: SocketAddr = "127.0.0.1:3000".parse()?;
serve(&addr, state).await?;
```

#### 自定义路由器

如果需要自定义路由或中间件，可以直接使用 `create_router`：

```rust
use coder::server::router::create_router;

let app = create_router(state);
// 可在此添加自定义中间件或路由
```

#### SessionManager

```rust
use coder::session::manager::SessionManager;

let manager = SessionManager::new();
let session = coder::session::Session::new();
manager.save(&session)?;                    // 保存
let loaded = manager.load(&session.id)?;    // 加载
let all = manager.list()?;                  // 列出所有
manager.delete(&session.id)?;               // 删除
```

#### ToolRegistry

```rust
use std::sync::Arc;
use coder::tool::ToolRegistry;

let registry = Arc::new(ToolRegistry::new());
let tools = registry.tool_defs();                       // 获取工具定义列表
let result = registry.execute("bash", args).await;       // 执行工具
```

#### Session 类型

```rust
use coder::session::Session;

let mut session = Session::new();
session.title = "My Session".to_string();
session.add_message(coder::ai::Message::user("Hello"));
println!("Messages: {}", session.message_count());
```

#### AI 消息构建

```rust
use coder::ai::{Message, ContentBlock, Role, GenerateConfig};

// 用户消息
let msg = Message::user("Hello");

// 系统消息
let sys = Message::system("You are a helpful assistant");

// 工具结果消息
let tool_result = Message::tool_result("tool_call_123", "command output");

// 自定义消息
let custom = Message {
    role: Role::User,
    content: vec![ContentBlock::Text { text: "Hi".to_string() }],
    name: None,
    tool_call_id: None,
};
```

#### Provider Trait

```rust
use coder::ai::{Provider, GenerateConfig, StreamEvent};
use tokio::sync::mpsc;

#[async_trait]
impl Provider for MyProvider {
    async fn chat_stream(
        &self,
        messages: &[Message],
        tools: &[ToolDef],
        config: &GenerateConfig,
    ) -> anyhow::Result<mpsc::Receiver<StreamEvent>> {
        // 实现流式聊天
    }
}
```

#### StreamEvent 处理

```rust
use coder::ai::StreamEvent;

while let Some(event) = receiver.recv().await {
    match event {
        StreamEvent::TextChunk(text) => print!("{}", text),
        StreamEvent::ToolCallStart(tc) => println!("\n[Tool: {}]", tc.name),
        StreamEvent::ToolCallResult { id, name, result } => {
            println!("[Result of {}]: {}", name, result);
        }
        StreamEvent::Done { stop_reason, usage } => {
            println!("\n[Done: {}, tokens: {:?}]", stop_reason, usage);
        }
        StreamEvent::Error(e) => eprintln!("[Error]: {}", e),
    }
}
```

---

## 9. 错误码

### HTTP 状态码

| 状态码 | 说明 | 典型场景 |
|--------|------|----------|
| `200 OK` | 请求成功 | 所有正常响应 |
| `400 Bad Request` | 请求参数错误 | 缺少必填字段、JSON 解析失败 |
| `404 Not Found` | 资源不存在 | 会话 ID 不存在 |
| `500 Internal Server Error` | 服务器内部错误 | Provider 不可用、IO 错误 |

### 错误消息示例

**会话不存在（404）：**

```json
{
  "error": "Session '550e8400-e29b-41d4-a716-446655440000' not found"
}
```

**请求参数错误（400）：**

```json
{
  "error": "missing field `message` at line 1 column 2"
}
```

**服务器内部错误（500）：**

```json
{
  "error": "Provider error: connection refused"
}
```

### SSE 错误事件

在 SSE 流式聊天中，错误通过 `error` 事件传递：

```
event: error
data: Provider unavailable: connection refused
```

### WebSocket 错误

WebSocket 连接中的错误通过 JSON 响应返回：

```json
{
  "type": "error",
  "data": {
    "message": "invalid message: missing field `type` at line 1 column 2"
  }
}
```

---

> 本文档基于 Coder v0.1.0 源码生成，覆盖 `server` 特性提供的全部 API 接口。
> 如有更新，请以源码为准。
