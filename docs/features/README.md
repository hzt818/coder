# Coder 功能文档

> 索引页面 — 各功能模块详细文档

## Phase 1 功能

| 模块 | 状态 | 说明 |
|------|------|------|
| [Team](./team.md) | 待完成 | 多智能体协作、任务分配、团队通信 |
| [Skill](./skill.md) | 待完成 | 技能系统、内置技能、技能注册与加载 |
| [Subagent](./subagent.md) | 待完成 | 子代理系统、7 种角色、上下文隔离 |
| [Memory](./memory.md) | 待完成 | 跨会话记忆、AutoDream 集成、检索 |
| [Storage](./storage.md) | 待完成 | SQLite 持久化、数据库迁移 |
| [LSP](./lsp.md) | 待完成 | 语言服务器协议、诊断注入、定义跳转 |
| [MCP](./mcp.md) | 待完成 | Model Context Protocol、Context7 集成 |

## Phase 2 功能

| 模块 | 状态 | 说明 |
|------|------|------|
| [Server](./server.md) | 待完成 | Axum HTTP API、WebSocket、SSE 流 |
| [Permission](./permission.md) | 待完成 | 策略引擎、权限评估、Allow/Deny/Ask |
| [Sync](./sync.md) | 待完成 | 云同步、上传/下载/双向同步 |
| [Voice](./voice.md) | 待完成 | 音频输入捕获、语音播放 |
| [OAuth](./oauth.md) | 待完成 | OAuth 2.0 授权码流程 |
| [Analytics](./analytics.md) | 待完成 | 事件追踪、使用量遥测 |
| [Computer](./computer.md) | 待完成 | 截图、鼠标控制、键盘输入 |
| [Worktree](./worktree.md) | 待完成 | Git worktree 管理 |

## 核心模块

| 模块 | 状态 | 说明 |
|------|------|------|
| [Audit](./audit.md) | 待完成 | 审计日志、工具执行记录 |
| [Checkpoint](./checkpoint.md) | 待完成 | 崩溃恢复、检查点写入 |
| [Compaction](./compaction.md) | 待完成 | 上下文压缩、Token 优化 |
| [Pricing](./pricing.md) | 待完成 | 成本跟踪、实时费用估算 |
| [Snapshot](./snapshot.md) | 待完成 | 快照回滚、Side-git 管理 |
| [Automation](./automation.md) | 待完成 | 定时任务、Cron 自动化 |
| [TaskManager](./task_manager.md) | 待完成 | 持久任务队列 |
| [LspHooks](./lsp_hooks.md) | 待完成 | LSP 后续编辑钩子 |

## 工具集

| 工具 | 状态 | 说明 |
|------|------|------|
| [apply_patch](./tool_apply_patch.md) | 待完成 | 统一 diff 应用 |
| [list_dir](./tool_list_dir.md) | 待完成 | Gitignore 感知目录列表 |
| [checklist](./tool_checklist.md) | 待完成 | 结构化 checklist 管理 |
| [fim_edit](./tool_fim_edit.md) | 待完成 | Fill-in-the-middle 编辑 |
| [github](./tool_github.md) | 待完成 | GitHub Issues/PR 操作 |
| [rlm](./tool_rlm.md) | 待完成 | Recursive Language Model REPL |
| [pr_attempt](./tool_pr_attempt.md) | 待完成 | PR 尝试追踪 |
| [snapshot_tool](./tool_snapshot_tool.md) | 待完成 | 快照恢复工具 |
| [automation_tool](./tool_automation_tool.md) | 待完成 | 自动化管理工具 |

## 其他

| 模块 | 状态 | 说明 |
|------|------|------|
| [Sandbox](./sandbox.md) | 待完成 | 本地/远程沙箱执行后端 |
| [i18n](./i18n.md) | 待完成 | 国际化框架、翻译系统 |
| [Commands](./commands.md) | 待完成 | 用户自定义斜杠命令 |
| [TUI](./tui.md) | 待完成 | 终端 UI 组件、Vim 模式、命令面板 |
| [Context7](./context7.md) | 待完成 | Context7 MCP 集成 |

---

*最后更新: 2026-05-05 — 文档框架已创建，内容待填充*