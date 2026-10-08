# S0 验证程序：宿主化系统已注册的预览处理器

这是 [SFM-复用系统预览处理器-可行性方案.md](../SFM-复用系统预览处理器-可行性方案.md) 里 **S0 阶段**的最小验证程序。

**它只回答一个问题**：你机器上注册的 WPS 预览器能不能被普通程序调用起来，把一个 `.docx` 真实地画进窗口，耗时多少。

不做 UI 集成、不做进程隔离、不做降级 —— 那些是 S1 以后的事。

> **本程序已按实测教训修正**：探测**两条注册路径**（`ProgID\shellex` 与 `<ext>\PersistentHandler`），不再只查前者。
> 背景：某测试机上 WPS 预览**实际可用**，但 `shellex` 与全局 `PreviewHandlers` 表**全空**——只查路径一会误判为"无处理器"。详见方案文档 2.5 节。

---

## ⚠️ 交付状态说明（请先读）

| 项目 | 状态 |
| --- | --- |
| 设计依据 | ✅ 基于 Microsoft 官方文档（预览处理器注册机制、`IPreviewHandler::SetWindow`） |
| 代码完成度 | ✅ 逻辑完整，可直接编译运行 |
| **是否已编译验证** | ❌ **未编译、未运行** —— 交付环境里 PowerShell 无法启动（`0xC0000142` DLL 初始化失败），`cargo`/`cl` 均不可用 |
| 风险点 | 见下方「已知不确定点」，预计需要 1–3 轮编译错误修正 |

**换句话说：这是一份"写完整了但没人跑过"的代码。** 首次 `cargo build` 大概率会有几处类型/签名不匹配需要调整，我在下面把最可能出问题的三处单独列了出来。

---

## 编译前提

1. **Rust**（含 MSVC 工具链 target）：`rustup default stable-msvc`
2. **Visual Studio Build Tools**（含 C++ 桌面开发 + Windows SDK）—— 链接 `shell32.lib`/`ole32.lib` 需要
3. **webview2 不需要**（这是个纯 Win32 程序，跟 Tauri 无关）

> **不想本地装这 5–6 GB？用云端编译。** 仓库已配好 GitHub Actions（`windows-latest` 自带 Rust + MSVC + Windows SDK）。
> 推送后自动编译，在 run 页面 **Artifacts** 区下载 `sfm-preview-spike.exe`，在装了 WPS 的本机运行即可。

## 编译与运行

```powershell
cd sfm-preview-spike

# 1) 先编译
cargo build --release

# 2) 无人值守地看探测报告（推荐先跑这个，不需要开窗）
.\target\release\sfm-preview-spike.exe --diag "C:\path\to\some.docx"

# 3) 机器可读输出（单行 JSON，供 CI 解析）
.\target\release\sfm-preview-spike.exe --diag --json "C:\path\to\some.docx"

# 4) 真正开窗预览
.\target\release\sfm-preview-spike.exe "C:\path\to\some.docx"
```

JSON 输出形如：

```json
{"ext":".docx","hkcu":"{0C7FEF07-...}","hklm":"{84F66100-...}","progid":"","effective":"{0C7FEF07-...}","registered":true,"verdict":"handler-found"}
```

| 字段 | 含义 |
| --- | --- |
| `hkcu` | HKCU 用户级注册值（**优先级最高**） |
| `hklm` | HKLM 机器级注册值（**可能是死值**，见下） |
| `progid` | 传统位置 `ProgID\shellex` |
| `effective` | 实际生效的 CLSID（HKCU > HKLM > ProgID） |
| `registered` | 是否在全局 `PreviewHandlers` 表中登记 |
| `verdict` | `handler-found` / `no-handler` |

---

## 先看 `--diag` 输出：怎么判断

`--diag` 会打印**两条路径**的完整关联链，**这是整个 S0 最有价值的部分**，因为它能把"没注册"和"注册了但调不通"区分开：

```
════════ 预览处理器探测报告 ════════
文件      : C:\path\to\some.docx
扩展名    : .docx
文件存在  : 是
ProgID    : WPS.Docx.6
路径一 shellex           : <未注册>
路径一b <ext>\shellex    : <未注册>
路径二 PersistentHandler : {D3B41FA1-01E3-49AF-AA25-1D0D824275AE}
全局 PreviewHandlers 表  : 未登记（仅对路径一有意义）

── 选中的 CLSID: {D3B41FA1-01E3-49AF-AA25-1D0D824275AE}
处理器名  : ...
InprocServer32: D:\...\Kingsoft\WPS Office\...\xxx.dll
             exists=True
════════════════════════════════════
```

对照下表读结果：

| 现象 | 含义 | 下一步 |
| --- | --- | --- |
| **两条路径都 `<未注册>`** | 本机确实没有可复用的渲染器 | 装 WPS/Office，或走**路线 B**（CLI 转 PDF） |
| **路径二有值、路径一为空** | **WPS 的典型形态**（本测试机就是这样） | 正常，继续开窗试调 |
| `ProgID: <未关联>` | 文件类型根本没关联 | 先修文件关联 |
| `exists=False` | **注册残留**（卸载没清干净）—— 高频坑 | 重装 WPS/Office |
| 报告正常但开窗失败 | 注册存在、**接口不匹配或调用不通** | 看下面的报错分类 |

### 开窗阶段的报错分类

| 报错 | 最可能原因 | 对策 |
| --- | --- | --- |
| `CoCreateInstance ... 失败` + `REGDB_E_CLASSNOTREG` | CLSID 注册损坏 | 重装 WPS/Office |
| `CoCreateInstance ... 失败` + `0x80040154` / 位数相关 | **32/64 位不匹配**（处理器只有 32 位实现） | 加 `--target i686-pc-windows-msvc` 编一版再试；或直接上 S1 的宿进程方案 |
| `SetWindow 失败` | 处理器要求特定宿主条件 | 上 S1 宿进程 |
| `DoPreview 失败` 或**卡住不返回** | WPS 处理器本身有问题 | **这正是要记录的关键数据** |
| 窗口一片空白但不报错 | 处理器画到了别的地方，或需要非零尺寸 | 把窗口拉大/最大化再看 |

---

## 已知不确定点（预计首次编译需要修）

这三处我无法在此环境验证，已在代码里做了保守选择，但可能需要调整：

### 1. `windows` crate 版本与 feature 门控

我指定了 `windows = "0.58"` 与这几个 feature：

```toml
"Win32_Foundation", "Win32_System_Com", "Win32_UI_Shell",
"Win32_UI_WindowsAndMessaging", "Win32_Graphics_Gdi"
```

**若报"找不到 `IPreviewHandler`"**，说明该接口在更细粒度的 feature 下，需补：

```toml
"Win32_UI_Shell_Common"      # 常见候选
```

快速排查方式：`cargo doc -p windows --no-deps` 后搜 `IPreviewHandler`，或在 `cargo build` 报错信息里点进 `windows` crate 的 feature 列表。

### 2. `AssocQueryStringW` 的签名与缓冲区语义

代码里采用"两段式调用"（先问长度、再取内容），并做了 `len` 的防御性处理。若签名不匹配，最可能报在 `PWSTR::null()` 与 `&mut len` 的类型上。

**另一种更稳的写法**（若编译不过，可换成这个思路）：直接给一个足够大的固定缓冲区，比如 `vec![0u16; 1024]`，把长度指针传 `&mut len`，一次调用拿结果 —— 牺牲一点严谨性换编译确定性。

### 3. `HBRUSH` / `COLORREF` 的构造

`WNDCLASSW` 里我用 `GetStockObject(WHITE_BRUSH).0` 包成 `HBRUSH`。若报类型不符，改成显式数值：

```rust
hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH(5isize),   // 白刷子句柄常量
```

或干脆设 `HBRUSH::default()`（窗口不刷背景，不影响验证目标）。

---

## 这个程序要记录什么数据

跑通后，**把下面四项填进方案文档的第 8 节「性能基线」**，它们直接决定后续投入：

| 指标 | 你的实测值 | 用途 |
| --- | --- | --- |
| `--diag` 是否解析出处理器 | | 判断这条路在目标环境是否可用 |
| 处理器由谁提供（名称/DLL） | | 判断是 WPS 还是 Office |
| **`DoPreview` 端到端耗时** | | 决定超时阈值（方案里暂定 3 秒） |
| 连续切换 10 个不同文档是否稳定 | | 决定是否需要 S1 的宿进程 |

### 决策规则（来自方案文档）

- **成功率 ≥ 80%、耗时 < 1.5 秒** → 按计划进 S1（宿进程化）
- **成功率中等、偶发卡死** → 仍然进 S1，但把看门狗阈值收紧，降级链权重加大
- **成功率 < 50%** → **放弃路线 A**，转向路线 B（WPS / LibreOffice CLI 转 PDF），这套代码作为诊断工具保留

---

## 下一步

S0 通过后，按方案文档第 9 节推进：

- **S1**：拆出 `sfm-preview-host.exe`，加 IPC、看门狗、`Unload`
- **S2**：渲染器注册表抽象（**可独立成 PR**，不含 Windows 专有代码）
- **S3**：接 Vue 信息面板（量坐标、loading 态）
- **S4**：快速查看 + 降级链 + 设置项 + 诊断面板
