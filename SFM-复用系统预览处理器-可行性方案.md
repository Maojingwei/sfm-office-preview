# SFM 复用系统预览处理器（路线 A）可行性方案

> 目标：让 Sigma File Manager 的**信息面板 / 快速查看**能够预览 Word / Excel / PPT，方式是**宿主化 Windows 已注册的 `IPreviewHandler`**——也就是 WPS 或 Office 已经装好的那个渲染器。
> 参照系：Windows 资源管理器预览窗格的做法。
> 适用范围：**仅 Windows**（COM + 注册表 + HWND 三者缺一不可）。
> 本文档为设计/可行性方案，非最终实现；路径与命名沿用 Tauri v2 项目惯例，落地时以实际代码结构为准。

---

## 1. 方案定位与核心结论

| 项目 | 结论 |
| --- | --- |
| 是否可行 | **可行**。任何进程都能当 `IPreviewHandler` 的宿主，不必是资源管理器 |
| 主程序体积代价 | **+0**。渲染能力来自用户已装的 WPS/Office |
| 保真度 | **高**（由 WPS/Office 自己渲染，与资源管理器预览窗格同级） |
| 跨平台 | **不可用**，只能作为 `#[cfg(windows)]` 分支 |
| 额外依赖 | 用户需装有注册预览处理器的应用（WPS / Office / OnlyOffice 等） |
| 最大风险 | 处理器是**第三方 COM 组件**，质量不可控（WPS 的处理器有大量配错先例），必须做进程隔离 + 超时降级 |

**核心机制一句话**：Windows 通过注册表把"文件类型 → 预览处理器 CLSID"关联起来，Shell 只负责查出 CLSID、创建 COM 对象、给它一个窗口句柄；渲染完全由处理器自己完成。我们把这个角色接过来即可。

---

## 2. 注册表解析：怎么找到"这个 .docx 该用哪个处理器"

### 2.1 关联链

> **⚠️ 本节为实测确认版**。早期版本只探测 `ProgID\shellex`，**在真实 WPS 环境上会完全失效**。
> 实测确认：WPS 把预览器注册在 **扩展名直属的 `ShellEx`** 下，且 **HKCU 覆盖 HKLM**。
> 完整实测数据见 [2.5](#25-实测确认wps-预览器的真实注册形态)。

```
【① 最高优先】用户级：HKCU\SOFTWARE\Classes\<ext>\ShellEx\{8895B1C6-…}
【② 次优先  】机器级：HKLM\SOFTWARE\Classes\<ext>\ShellEx\{8895B1C6-…}
【③ 传统位置】HKCR\<ext> → ProgID\shellex\{8895B1C6-…}

        （三者的默认值都指向）{CLSID}
                      └─ CLSID\{那个 GUID}
                           ├─ (默认值) = 显示名
                           ├─ InprocServer32   → 进程内 DLL        （64 位处理器走这条）
                           ├─ InprocHandler32  → ole32.dll          （无 InprocServer32 时的跨位数/代理形态）
                           └─ AppID            → 代理宿主配置
```

**三个必须记住的坑**：

1. **注册点在扩展名上，不在 ProgID 上。** 找 `ProgID\shellex` 会一无所获。
2. **HKCU 优先于 HKLM。** 本机 `HKLM\.docx\ShellEx` 指向 `{84F66100-…}`，但**该 CLSID 在注册表里根本不存在**；真正生效的是 HKCU 的 `{0C7FEF07-…}`。**只看 HKLM 会拿到一个死值。**
3. **32 位注册在 `WOW6432Node` 下。** 本机 WPS 预览器只在 `HKLM\SOFTWARE\WOW6432Node\Classes\CLSID` 下注册，且**没有 `InprocServer32`、只有 `InprocHandler32 = ole32.dll`**——说明它是 32 位组件，64 位宿主必须走 DCOM 代理激活。

`{8895B1C6-B41F-4C1C-A562-0D564250836F}` 是预览处理器的固定扩展点 GUID；`{E357FCCD-A995-4576-B01F-234630154E96}` 是缩略图处理器（`IThumbnailProvider`）的，**别搞混**。
另注：`<ext>\PersistentHandler` 是**搜索索引用的文本提取处理器**（如 "Office Open XML Format Word Persistent Handler"），**不是预览处理器**，别误当预览来源。

### 2.2 推荐解析方式

**不要自己爬注册表**，用系统提供的解析函数：

```rust
// 伪代码：用 AssocQueryString 拿 ProgID 对应的预览处理器 CLSID
// ASSOCF_NONE, ASSOCSTR_SHELLEXTENSION, pszAssoc=".docx",
// pszExtra="{8895B1C6-B41F-4C1C-A562-0D564250836F}" → 返回 "{CLSID}"
fn resolve_preview_handler_clsid(ext: &str) -> Option<String>
```

理由：`AssocQueryString` 会正确处理用户级/机器级关联差异、`.docx` → ProgID 的多级间接、以及 Windows 10+ 的用户选择覆盖（UserChoice），自己爬注册表很容易拿到过时或错误的值。

### 2.3 解析结果缓存

- 按**扩展名**（而非文件路径）缓存，一个会话内注册表基本不变；
- 记录解析失败（`None`）也需要缓存，避免每个文件都查一次；
- 提供"重新探测"入口（用户装了 WPS 之后不必重启应用）。

### 2.4 探测阶段的额外校验（重要）

配置阶段先做一次"体检"，避免运行期才发现不通：

1. 该 CLSID 是否能解析到实现（`InprocServer32` 或 `LocalServer32`）；
2. **路径一**额外检查：CLSID 是否存在于 `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\PreviewHandlers` 全局登记表（**注意：此表只覆盖路径一，路径二不登记在这里，不能用它判断路径二是否有效**）；
3. `InprocServer32` 指向的 DLL **文件是否存在**（WPS 卸载残留常导致注册表项还在、DLL 已经没了）；
4. 冷启动试调一次（用一个极小的样例文件），能出画面才算"可用"。

体检结果直接决定这个格式走"系统处理器"还是走降级链。

### 2.5 实测确认：WPS 预览器的真实注册形态

> 本节是**逐轮实测修正后的最终结论**。早期版本先后误判过两次（先只查 `ProgID\shellex`，后误信 `PersistentHandler`），均已纠正。**记录在此以防重犯。**

**测试环境**：Windows AMD64（64 位）、WPS Office 12.1.0.28505、**未安装 Microsoft Office**（只剩 ProgID 残留）。

**探测结果总表**

| 探测位置 | `.docx` 结果 |
| --- | --- |
| `HKCR\.docx` 的 ProgID | `WPS.Docx.6` |
| `HKCR\WPS.Docx.6\shellex` | **不存在** |
| `HKCR\SystemFileAssociations\.docx\shellex\{8895B1C6-…}` | **未注册** |
| `AssocQueryString(ASSOCSTR_SHELLEXTENSION, ".docx", {8895B1C6-…})` | **返回空**（不可靠，见下） |
| **`HKLM\SOFTWARE\Classes\.docx\ShellEx\{8895B1C6-…}`** | `{84F66100-FF7C-4fb4-B0C0-02CD7FB668FE}` ⚠️ **死值** |
| **`HKCU\SOFTWARE\Classes\.docx\ShellEx\{8895B1C6-…}`** | **`{0C7FEF07-DCD9-4120-9647-D1CE32F289CD}`** ✅ **生效** |
| `HKCR\.docx\PersistentHandler` | `{D3B41FA1-…}` → **搜索索引用，非预览** |
| 全局 `PreviewHandlers` 表（HKLM 及 WOW6432Node） | **含 WPS 三项** ✅ |

**全局表内容（关键证据）**

```
{0C7FEF07-DCD9-4120-9647-D1CE32F289CD}   WPS文字 预览器
{E260F96C-8EF4-4C24-A2B9-455F1D116531}   WPS表格 预览器
{A1BBCFD9-B54C-443D-BC56-0BC3840120DB}   WPS演示 预览器
```

**CLSID 定位结果**

| CLSID | 注册位置 | 形态 |
| --- | --- | --- |
| `{0C7FEF07-…}` WPS文字 | **仅** `HKLM\SOFTWARE\WOW6432Node\Classes\CLSID` | 无 `InprocServer32`，仅 `InprocHandler32 = ole32.dll` |
| `{E260F96C-…}` WPS表格 | 同上 | 同上 |
| `{A1BBCFD9-…}` WPS演示 | 同上 | 同上 |
| `{84F66100-…}`（HKLM 指向） | **三个视图下均不存在** | 死值 |

**三条硬结论**

1. **注册在扩展名直属的 `ShellEx` 下**，不在 `ProgID\shellex`。这是最容易踩的坑——查错层级会得出"没有处理器"的错误结论。
2. **HKCU 覆盖 HKLM，且不能只看 HKLM**：`HKLM\.docx\ShellEx` 指向的 `{84F66100-…}` **在注册表里根本不存在**，是死值；真正生效的 HKCU 值 `{0C7FEF07-…}` 才是 WPS 预览器。
3. **WPS 预览器是 32 位组件**：只注册在 `WOW6432Node`（32 位视图）、且只有 `InprocHandler32 = ole32.dll`、没有 `InprocServer32`。→ **64 位宿主必须走 DCOM 代理激活（`CLSCTX_LOCAL_SERVER`），用 `CLSCTX_INPROC_SERVER` 必然失败。**

**两个被我误判过的陷阱（务必记住）**

- ❌ **`<ext>\PersistentHandler` 不是预览处理器**。它的名字是 "Office Open XML Format Word Persistent Handler"，用途是**给 Windows Search 提取文本做索引**。（`.doc` 那个 `{98de59a0-…}` = "Microsoft Office Persistent Handler" 同理。）把预览指望在它身上是错的。
- ❌ **`AssocQueryString(ASSOCSTR_SHELLEXTENSION)` 在本机返回空**，尽管注册表里确实有值。→ **不能只依赖它**，必须直接读注册表兜底。（可能原因：该 API 走的是与 HKCU `Classes` 合并视图不同的解析路径。此点未深究，但不影响结论——**两条路都走**即可。）

**旁证**：WPS 的 PDF 预览出错提示为 "PDF Preview Handler 中出现错误"，且全局表里 WPS 三项的命名格式与系统其它预览器（"Microsoft PDF Previewer" 等）完全一致 → **WPS 实现的是标准 `IPreviewHandler` 契约**。这使宿主方案可以采用标准时序。

### 2.6 宿主接口与宿主形态

**接口**：由上节旁证推断为 `IPreviewHandler`（标准契约），但**代码仍按三接口依次降级实现**，不押注：

```
CoCreateInstance(CLSID, CLSCTX_LOCAL_SERVER | CLSCTX_INPROC_SERVER)
  ├─ IPreviewHandler            → SetWindow / SetRect / DoPreview   （首选路径）
  ├─ IInitializeWithFile / IInitializeWithItem   （数据源初始化，按可用性降级）
  ├─ IPersistFile::Load(path, STGM_READ)         （若无 IPreviewHandler）
  └─ IPersistStream::Load(IStream)               （最后的回退）
```

**宿主形态（32/64 位问题）**：这是本方案在真实环境中的**主要工程约束**。

| 宿主 | `CLSCTX` | 结果 |
| --- | --- | --- |
| 64 位进程 | `INPROC_SERVER` | ❌ **必然失败**（32 位 DLL 无法载入 64 位进程） |
| 64 位进程 | `LOCAL_SERVER` | ✅ 走 DCOM 代理宿主（系统已有 `SysWOW64\prevhost.exe` 等代理配置） |
| **32 位宿进程** | `INPROC_SERVER` | ✅ 最直接，无跨进程开销 |

系统里已有的代理宿主配置（实测 `AppID` 表）：

```
{6D2B5079-2F0B-48DD-AB7F-97CEC514D30B}  DllSurrogate = C:\WINDOWS\system32\prevhost.exe    （64 位）
{534A1E02-D58F-44F0-B58B-36CBED287C7C}  DllSurrogate = C:\Windows\SysWOW64\prevhost.exe    （32 位）
```

→ **方案第 3 节的"自建宿进程"因此不仅是可靠性选择，更是位数适配的必需项**：宿进程**应当编译为 32 位**，才能用 `INPROC_SERVER` 直接载入 WPS 预览器，避免依赖 DCOM 代理。

---

## 3. 进程与窗口架构（本方案最关键的部分）

### 3.1 为什么必须独立宿主进程

三类硬约束把方案推向"专用宿进程"：

1. **崩溃隔离**：处理器由第三方提供，挂死/崩溃是常态。跑在 SFM 主进程里，一次 WPS 处理器崩溃就会带走整个文件管理器。
2. **位数匹配**：SFM 是 64 位；若处理器只注册了 32 位进程内实现，主进程内 `CoCreateInstance` 直接失败。独立进程可以按需起 32 位或 64 位宿主。
3. **线程模型**：多数处理器要求 **STA** + 消息泵。放在 WebView2 的主 UI 线程上会互相干扰。

顺带说明：资源管理器本身就是这么做的（处理器跑在 `prevhost.exe` 里，窗口却嵌在资源管理器 UI 上），所以**跨进程窗口父子关系是 Windows 的既定用法**，不是黑魔法。

### 3.2 两个候选实现形态

| | **A1. 借用系统代理宿主（Prevhost）** | **A2. 自建宿进程（推荐）** |
| --- | --- | --- |
| 做法 | 64 位进程用 `CLSCTX_LOCAL_SERVER`，让 DCOM 自动进 `SysWOW64\prevhost.exe` | 自己编译 **32 位** `sfm-preview-host.exe`，进程内直接创建 COM 对象 |
| 代码量 | **最少** | 中（多一个可执行文件 + IPC 协议） |
| 位数问题 | 系统代理处理，但有 DCOM 启动开销与不确定性 | **宿进程编成 32 位即可用 `INPROC_SERVER` 直接载入，最直接** |
| 崩溃影响 | 不会带崩 SFM | 不会带崩 SFM |
| 可控性 | 低——无法设自己的超时、拿不到失败原因、窗口归属难管理 | **高**——可加看门狗、日志、强制超时、优雅降级 |
| 调试 | 难（进程是系统的） | 容易 |

**结论：A1 用来做第一阶段验证（一两天就能出画面），A2 作为正式形态。**

> **⚠️ 位数结论已由实测确定**：WPS 的预览器（`{0C7FEF07-…}` 等）只注册在 `WOW6432Node`、只有 `InprocHandler32`，**是 32 位组件**。
> 因此 **A2 的宿进程应当编译为 32 位**（`i686-pc-windows-msvc`），用 `CLSCTX_INPROC_SERVER` 直接载入，避免依赖 DCOM 代理的启动开销与失败模式。
> 若坚持只出 64 位宿主，则必须用 `CLSCTX_LOCAL_SERVER` 走代理——可行但更脆弱。

### 3.3 A2 的进程/窗口结构

```
┌─────────────────────── Sigma File Manager (64-bit, Tauri) ───────────────────────┐
│                                                                                  │
│  Vue 信息面板 / 快速查看                                                          │
│    └─ 测量出预览区域 → 屏幕坐标 → IPC 交给 Rust                                   │
│                                                                                  │
│  Rust: preview_handler 模块                                                       │
│    ├─ resolve_preview_handler_clsid(ext)        ← 第 2 节                         │
│    ├─ host_pool: 复用/回收宿进程                                                  │
│    └─ IPC 客户端（stdin/stdout JSON 行 或 命名管道）                               │
│                                                                                  │
│  Tauri 主窗口 HWND ──────────────────────────────┐                               │
└──────────────────────────────────────────────────┼───────────────────────────────┘
                                                   │ SetParent(host_hwnd, main_hwnd)
                                                   │ SetWindowPos(位置/大小)
┌──────────────────────────────────────────────────┼───────────────────────────────┐
│  sfm-preview-host.exe（按位数选择 32/64）         ▼                               │
│    ├─ STA 线程 + 消息泵（CoInitializeEx(COINIT_APARTMENTTHREADED)）               │
│    ├─ CoCreateInstance(CLSID) → IPreviewHandler                                  │
│    ├─ IInitializeWithFile / Item / Stream                                        │
│    ├─ CreateWindowEx(WS_CHILD | WS_VISIBLE, parent = 自己的顶层窗口)               │
│    ├─ IPreviewHandler::SetWindow(child_hwnd, rect) · SetRect · DoPreview          │
│    └─ 看门狗：DoPreview / SetRect 超时 → Unload + 上报失败                         │
└──────────────────────────────────────────────────────────────────────────────────┘
```

要点：

- **宿进程先创建自己的顶层窗口**，再把该窗口 `SetParent` 到 SFM 主窗口下变成子窗口。这样窗口的过程（WndProc）仍然在宿进程里执行，渲染和输入都不跨进程调用，只有窗口管理是跨进程的。
- **坐标来源是 web 层**：Vue 量出信息面板预览区域的屏幕坐标（`getBoundingClientRect()` + 窗口位置），通知 Rust 去摆放那个窗口。DPI 缩放必须在这个换算里处理。
- **Z 序**：用 `SetWindowPos(..., HWND_TOP, ...)` 保证预览窗口盖在 WebView2 之上；隐藏时移出可见区域或 `ShowWindow(SW_HIDE)`。
- **交互取舍**：是让预览可滚动/可缩放（把鼠标事件交给处理器，体验好），还是只做静态展示（web 层盖一层遮罩，自己处理滚动，简单可控）——建议**先做静态展示**，处理器只负责画，滚动由 web 层通过改变 `SetRect` 或让处理器自己滚两种方式择一。

---

## 4. 处理器调用时序与参数

```
1. CoInitializeEx(NULL, COINIT_APARTMENTTHREADED)     // STA，必须
2. CoCreateInstance(clsid, NULL, CLSCTX_LOCAL_SERVER | CLSCTX_INPROC_SERVER,
                    IID_IPreviewHandler, &handler)
3. 初始化数据源，按优先级尝试（按接口可用性降级）：
     QueryInterface(IInitializeWithFile)   → Initialize(path, STGM_READ)
     QueryInterface(IInitializeWithItem)   → Initialize(shell_item, STGM_READ)
     QueryInterface(IInitializeWithStream) → Initialize(stream, STGM_READ)
4. QueryInterface(IObjectWithSite) → SetSite(NULL)     // 可选，部分处理器需要
5. handler.SetWindow(child_hwnd, &rect)
6. handler.SetRect(&rect)
7. handler.DoPreview()
8. 交互期间：面板变化 → handler.SetRect(&new_rect)
9. 切换文件/关闭面板：handler.Unload()  → 释放接口 → 复用宿进程
```

### 关键实现注意点

- **`SetWindow` 给的是"画布"**：处理器自己往这个 HWND 上画，宿主不要在这个窗口上做自绘，否则会被覆盖。
- **`IInitializeWithFile` 常失败**：不少处理器只实现 `IInitializeWithStream` 或 `IInitializeWithItem`（尤其是邮件附件型处理器），必须实现完整降级链。
- **`DoPreview` 是同步阻塞的**：大文档可能几百毫秒到数秒。宿进程里必须**另起工作线程或至少加看门狗**，且 SFM 的 UI 要立刻显示 loading 态。
- **`Unload` 要真的调用**：不调用会导致 WPS 进程驻留、文件句柄不释放（用户会遇到"文档被占用"）。
- **文件被占用**：WPS/Office 可能持有文件锁；预览失败时给出人话提示，并允许用 `STGM_READ | STGM_SHARE_DENY_NONE` 之类的共享模式重试。

---

## 5. 与现有架构的对接

### 5.1 新增组件

```
src-tauri/
  src/
    preview_handlers/
      mod.rs            # 对外统一入口：can_preview(ext) / attach(rect) / detach()
      registry.rs       # AssocQueryString 解析 + 缓存 + 体检
      host_process.rs   # 宿进程生命周期、IPC、看门狗、进程池
  binaries/
    sfm-preview-host-<target-triple>.exe   # 侧车（Tauri externalBin）
src/
  components/InfoPanel/
    SystemPreviewSurface.vue   # 占位 + 量坐标 + loading/失败态
```

宿进程作为 Tauri **侧车**分发（`tauri.conf.json` 的 `bundle.externalBin`，按目标三元组命名）。**不要捕获它的 stdout 管道**——用文件/命名管道传状态，避免管道相关限制。

### 5.2 类型分发的抽象（建议顺手做掉）

现在的"类型 → 渲染器"是硬编码的。这次改造顺便抽成一张**渲染器注册表**：

```
文件类型 → 候选渲染器列表（按优先级）
  ├─ system-preview-handler   （Windows，本方案）
  ├─ builtin-pdf / builtin-media / builtin-text
  └─ builtin-docx-js / builtin-xlsx-js  （纯前端降级）
```

好处：本方案只是列表里的一项，将来加 LibreOffice 转换、加扩展提供的渲染器都是往表里插一行，不用再动分发逻辑。同时这也让 PR 更容易被上游接受——它引入的是通用机制，不是"给 WPS 做适配"。

### 5.3 降级链（必须实现）

```
1. 系统预览处理器（本方案）         ← 有注册且体检通过
2. 内置 PDF / 媒体 / 文本渲染器      ← 已有能力
3. 纯前端 docx-preview / SheetJS    ← 覆盖 Word、Excel（PPT 放弃）
4. 静态图标占位                     ← 保持现状，永不报错
```

触发降级的条件：解析失败、体检不通过、`DoPreview` 超时（建议 3 秒起，可配）、宿进程崩溃、连续失败达阈值（此后该扩展名本次会话直接走降级，不再重试）。

---

## 6. 设置项与诊断（决定这个功能好不好用）

| 设置项 | 建议默认 | 说明 |
| --- | --- | --- |
| 使用系统预览器预览文档 | 开 | 总开关；关闭后走内置渲染器 |
| 预览加载超时 | 3 秒 | 超时即降级 |
| 预览窗口保留策略 | 会话内复用宿进程 | 兼顾启动速度与内存 |
| 诊断面板 | — | 显示"当前 .docx 由哪个处理器提供（CLSID + DLL 路径）"、提供"测试系统预览器"按钮、导出日志 |

诊断能力建议第一天就做——因为用户环境差异极大，"你那台机器上到底解析到了什么"是最常见的支持问题。

---

## 7. 风险清单与对策

| 风险 | 影响 | 对策 |
| --- | --- | --- |
| **WPS 预览处理器注册错乱**（有键无 DLL、键指向旧版本、位数不符） | 预览不显示或报错 | 配置期体检 + 运行期超时 + 降级链；诊断面板暴露 CLSID 便于定位 |
| 处理器挂死 | 面板卡住 | 宿进程 + 看门狗 + 强制 `Unload` + 杀进程重建 |
| 位数不匹配 | `CoCreateInstance` 失败 | 按需提供 32/64 两版宿进程，或先走 A1 让系统决定宿主 |
| 文件被 WPS/Office 锁定 | 预览失败或提示占用 | 共享模式打开、失败给明确文案 |
| 大文档渲染慢 | 体感差 | 立即 loading 态、可取消、缓存"上次成功"结果 |
| DPI / 多显示器缩放 | 窗口位置错位 | 坐标换算用 `GetDpiForWindow`，监听 `WM_DPICHANGED` |
| 深色主题不匹配 | 视觉突兀 | 部分处理器读系统主题；必要时提供"预览区域白底/跟随主题"开关 |
| 上游不接受 | 白干 | 按 5.2 做成通用机制，并拆成两个独立 PR（见第 9 节） |
| 跨平台用户误解 | 反馈噪音 | 明确标注 Windows-only，Linux/macOS 走降级链 |

---

## 8. 验收标准与测试矩阵

**功能验收**

- [ ] `.docx` / `.xlsx` / `.pptx` 在信息面板出现真实内容（非图标）
- [ ] 连续切换 20 个文档，无残留窗口、无文件句柄泄漏
- [ ] 关掉信息面板后，WPS 没有残留的预览进程
- [ ] 卸载 WPS（或删除其 DLL）后，应用不崩、自动降级为图标

**异常验收**

- [ ] 加密文档、损坏文档、被占用的文档 → 均给出可读提示，不卡死
- [ ] 手动杀掉宿进程 → 自动重建，最多连续失败 N 次后本次会话放弃该系统处理器
- [ ] 150% / 200% DPI、多显示器、窗口最大化 → 预览区域对齐

**性能基线**

- [ ] 首次预览 .docx 端到端延迟（记录实测数字，作为超时阈值依据）
- [ ] 宿进程常驻内存增量
- [ ] 信息面板频繁开关时的 CPU 抖动

---

## 9. 分阶段实施建议

| 阶段 | 内容 | 产出 | 估时 |
| --- | --- | --- | --- |
| **S0. 技术验证（先做这个）** | 用 A1 形态（直接 `CoCreateInstance`，借 Prevhost）跑通 `.docx`：解析 CLSID → 创建对象 → 给一个普通 Win32 子窗口 → `DoPreview`。**不做 UI 集成** | 一张"WPS 处理器在本机真的能画出内容"的截图 + 实测延迟 | 1–2 天 |
| **S1. 宿进程化** | 拆出 `sfm-preview-host.exe`，加 IPC、看门狗、Unload | 崩溃可控的最小实现 | 3–5 天 |
| **S2. 渲染器注册表抽象** | 把类型分发抽成候选列表机制，本方案作为其中一项 | 通用机制（**可独立成 PR**） | 3–5 天 |
| **S3. UI 集成** | Vue 侧占位/量坐标/loading/失败态，接信息面板 | 可用版本 | 3–5 天 |
| **S4. 快速查看 + 降级链 + 设置项 + 诊断** | 补齐体验与兜底 | 可发布版本 | 1 周 |

**S0 是决策点**：如果 S0 发现 WPS 处理器在你的目标环境里调用成功率低（WPS 版本差异导致），就应该把资源转向"路线 B（WPS/LibreOffice CLI 转 PDF）"，而不是继续投入 S1。

**上游 PR 切分建议**（提高接受率）：

- **PR 1**：S2 的渲染器注册表抽象（不含任何 Windows 专有代码）——纯架构改进，最容易过。
- **PR 2**：Windows 系统预览处理器宿主（S0+S1+S3），作为注册表里的一个条目接入。
- 这样即使 PR 2 因"仅 Windows / 维护成本"被拒，PR 1 也已经把口子留好，社区能自行补插件。

---

## 10. 一句话总结

**技术上完全可行，最大的红利是"零体积代价换来 WPS 级保真度"，最大的成本是"必须做进程隔离和超时降级"——因为你要伺候的是一堆质量参差的第三方 COM 组件，而不是一个稳定 API。** 先用 1–2 天做 S0 验证在你目标机器上的实际调用成功率，再决定是否全量投入。
