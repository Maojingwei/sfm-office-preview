# SFM Office 预览可行性研究

围绕 [Sigma File Manager](https://github.com/aleksey-hoffman/sigma-file-manager) 能否预览 Word / Excel / PPT 的可行性研究，以及复用 Windows 系统预览处理器（WPS / Office）的技术验证。

## 结论摘要

Sigma File Manager v2.2.0 **不支持** `.docx` / `.xlsx` / `.pptx` 预览（信息面板只显示图标占位）。本文研究的是**能否复用系统里已注册的 WPS/Office 预览器**来补上这个能力。

**关键实测结论**（Windows AMD64 + WPS Office 12.1.0.28505）：

| 发现 | 内容 |
| --- | --- |
| 注册位置 | 预览器注册在 **扩展名直属的 `ShellEx`** 下，**不在 `ProgID\shellex`** 下 |
| 优先级 | **HKCU 覆盖 HKLM**；本机 `HKLM\.docx\ShellEx` 指向的 CLSID **根本不存在**（死值） |
| 真正的处理器 | `{0C7FEF07-…}` WPS文字 / `{E260F96C-…}` WPS表格 / `{A1BBCFD9-…}` WPS演示 |
| **位数** | WPS 预览器**只注册在 `WOW6432Node`（32 位）**，无 `InprocServer32`，仅 `InprocHandler32 = ole32.dll` |
| 接口 | 推断为标准 `IPreviewHandler`（全局表命名格式与系统其它预览器一致） |

→ **64 位宿主必须走 `CLSCTX_LOCAL_SERVER`（DCOM 代理）；宿进程编成 32 位才能直接 `INPROC_SERVER` 载入。**

## 目录结构

```
.
├── Sigma-File-Manager-使用说明.md              # SFM 完整中文使用说明（v2.2.0）
├── SFM-复用系统预览处理器-可行性方案.md          # 路线 A 的设计与可行性方案
├── sfm-preview-spike/                         # S0 验证程序（Rust）
│   ├── src/main.rs
│   ├── Cargo.toml
│   └── README.md
└── .github/workflows/build.yml                # 云端编译（windows-latest 自带 Rust+MSVC）
```

## S0 验证程序

`sfm-preview-spike` 是最小验证程序，只回答一个问题：**系统里注册的 WPS 预览器能不能被普通程序调用起来**。

```powershell
# 探测模式：打印三处注册点的解析结果（不需要看画面）
.\sfm-preview-spike.exe --diag "C:\path\to\a.docx"

# 机器可读输出（供 CI 解析）
.\sfm-preview-spike.exe --diag --json "C:\path\to\a.docx"

# 真正开窗预览并打印 DoPreview 耗时
.\sfm-preview-spike.exe "C:\path\to\a.docx"
```

## 云端编译

本项目用 GitHub Actions 编译（`windows-latest` runner 自带 Rust + MSVC + Windows SDK，**本地无需安装 5–6 GB 工具链**）。

- 推送后自动触发，也可在 Actions 页面手动 `workflow_dispatch`
- 产物：`sfm-preview-spike.exe` + 诊断输出，在 run 页面的 **Artifacts** 区下载
- **注意**：CI runner 上**没有 WPS**，所以 `--diag` 预期返回"三处注册点均为空"。CI 验证的是**能否编译 + 注册表读取逻辑正确**；**真实预览必须在本机（装有 WPS）运行 exe 验证**

## 本地编译（可选）

若不想用云端，需要自行安装：Rust（MSVC 工具链）+ VS Build Tools（含 C++ 与 Windows SDK），合计约 5–6 GB。

```powershell
cd sfm-preview-spike
cargo build --release
```

## 相关文档与依据

- [注册预览处理器（Microsoft）](https://learn.microsoft.com/zh-cn/previous-versions//bb776868(v=vs.85))
- [IPreviewHandler::SetWindow](https://learn.microsoft.com/windows/win32/api/shobjidl_core/nf-shobjidl_core-ipreviewhandler-setwindow)
- [Tauri 侧车（externalBin）](https://v2.tauri.app/develop/sidecar/)
- SFM 发布说明：[v2.2.0](https://newreleases.io/project/github/aleksey-hoffman/sigma-file-manager/release/v2.2.0) · [v2.1.0](https://newreleases.io/project/github/aleksey-hoffman/sigma-file-manager/release/v2.1.0)
