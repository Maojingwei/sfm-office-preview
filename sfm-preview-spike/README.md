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
| **编译** | ✅ **已在 GitHub Actions `windows-latest` 上编译通过**（clippy 也过） |
| **运行** | ✅ **已实测**：`--diag` 与预览模式均在本机跑通 |
| **视觉确认** | ⏳ **待你在本机看一眼窗口内容**（CI 无桌面，本会话无审批权限弹窗） |

### S0 实测结论（Windows AMD64 + WPS Office 12.1.0.28505）

| 指标 | 实测值 |
| --- | --- |
| `.docx` 生效 CLSID | `{0C7FEF07-DCD9-4120-9647-D1CE32F289CD}`（"WPS文字 预览器"） |
| 解析来源 | `HKCU\SOFTWARE\Classes\.docx\ShellEx\{8895B1C6-…}` |
| HKLM 候选 | `{84F66100-FF7C-4FB4-B0C0-02CD7FB668FE}` → **三个视图下均不存在，死值** |
| 实现形态 | 仅注册于 `WOW6432Node`，只有 `InprocHandler32 = ole32.dll` → **32 位组件** |
| 初始化接口 | `IInitializeWithFile` |
| 激活方式 | `CLSCTX_LOCAL_SERVER \| CLSCTX_INPROC_SERVER`（64 位宿主走 DCOM 代理） |
| **`DoPreview` 端到端耗时** | **77 ms（热）/ 2554 ms（冷启动首次）** |
| 结果 | ✅ `DoPreview` 返回成功，窗口创建成功（`MainWindowHandle` 非空，标题正确） |

**结论：路线 A 在真实环境可跑通。** 三个此前预判的风险全部得到实测确认：

1. 注册点在**扩展名直属的 `ShellEx`**，不在 `ProgID\shellex`（查错层级会误判为"无处理器"）
2. **HKCU 覆盖 HKLM**，且 HKLM 指向的是**死值**——遇到第一个值就用会失败（已加存在性校验 + 回退）
3. WPS 预览器是 **32 位**，64 位宿主必须走 `CLSCTX_LOCAL_SERVER`

### 你需要做的最后一步

下载 `sfm-preview-spike.exe`，在**装有 WPS 的 Windows** 上运行：

```powershell
# 看到窗口内容即 S0 通过
.\sfm-preview-spike.exe "C:\path\to\any.docx"

# 只看探测结果，不开窗
.\sfm-preview-spike.exe --diag "C:\path\to\any.docx"

# 机器可读输出
.\sfm-preview-spike.exe --diag --json "C:\path\to\any.docx"
```

预期：弹出窗口并由 **WPS** 渲染出文档内容，控制台打印 `端到端耗时: xxx ms` 与 `DoPreview 已返回`。

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

## `--diag` 输出：怎么判断

`--diag` 打印**三处注册点**的解析链，**这是整个 S0 最有价值的部分**——它能把"没注册"、"注册了但是死值"、"注册了且可用"三种情况区分开。

**本机实测输出**（WPS Office 12.1.0.28505）：

```
════════ 预览处理器探测报告 ════════
文件      : C:\Users\...\real.docx
扩展名    : .docx
文件存在  : 是
ProgID    : WPS.Docx.6
① HKCU\...\.docx\ShellEx   : {0C7FEF07-DCD9-4120-9647-D1CE32F289CD}
② HKLM\Classes\.docx\ShellEx: {84F66100-FF7C-4FB4-B0C0-02CD7FB668FE}
③ ProgID\shellex             : <无>
   （生效优先级：① > ② > ③）
全局 PreviewHandlers 表      : 未登记

── 生效 CLSID: {0C7FEF07-DCD9-4120-9647-D1CE32F289CD}

宿主提示：若该 CLSID 无 InprocServer32 且仅有 InprocHandler32/仅注册于 WOW6432Node,
则它是 32 位组件，64 位宿主必须用 CLSCTX_LOCAL_SERVER 走 DCOM 代理（本程序已如此设置）。
════════════════════════════════════
```

**注意 ② 是死值**：`{84F66100-…}` 在三个注册表视图下都不存在（Office 卸载残留），程序会打印
`[i] 跳过候选 …（该 CLSID 未在注册表中注册，属残留死值）` 并回退到 ①。**这是必须的健壮性处理**——遇到第一个值就用会直接失败。

对照下表读结果：

| 现象 | 含义 | 下一步 |
| --- | --- | --- |
| 三处全 `<无>` | 本机确实没有可复用的渲染器 | 装 WPS/Office，或走**路线 B**（CLI 转 PDF） |
| **只有 ①（HKCU）有值** | **WPS 的典型形态**（本测试机就是这样） | 正常 |
| ② 有值但被"跳过候选" | 该 CLSID 是**死值**（卸载残留） | 正常；若三处都是死值则同上一条 |
| `ProgID: <未关联>` | 文件类型根本没关联 | 先修文件关联 |
| 全部候选都被跳过 | 注册表指向的处理器都不存在 | 重装 WPS/Office |

### 开窗阶段的报错分类

| 报错 | 最可能原因 | 对策 |
| --- | --- | --- |
| `REGDB_E_CLASSNOTREG` | CLSID 在当前位数视图下不存在 | 检查候选是否死值；或用 `CLSCTX_LOCAL_SERVER` |
| `CO_E_SERVER_EXEC_FAILURE` / `0x80080005` | DCOM 代理宿主启动失败 | 确认设备激活服务正常；或改 32 位宿主 |
| `SetWindow 失败` | 处理器要求特定宿主条件 | 上 S1 宿进程 |
| `DoPreview 失败` 或**卡住不返回** | 处理器本身有问题 | 这正是要记录的关键数据 |
| 窗口空白但不报错 | 处理器画到了别处，或窗口尺寸为 0 | 拉大窗口；检查 `SetRect` |

---

## `windows` crate 的 API 陷阱（实测踩过的坑）

代码用的是 `windows = "0.58"`。**该版本的签名与 0.62 文档不一致**，以下都是实测报错后修正的，留作参考：

| 项目 | 0.58 的实际情况 |
| --- | --- |
| `GetModuleHandleW` 返回 | `HMODULE`（`*mut c_void` 包装），需转 `HINSTANCE`；**`HINSTANCE(m.0)` 即可**，不能传整数 |
| `CreateWindowExW` 的 `hinstance` | 裸 `HINSTANCE`，**不是** `Option<HINSTANCE>` |
| `CreateWindowExW` 的 `hwndparent` | 裸 `HWND`，可直接传 |
| `RegOpenKeyExW` 的 `ulOptions` | 裸 `u32`（0.62 是 `Option<u32>`） |
| `RegQueryValueExW` 的 `lpreserved` | `Option<*const u32>` |
| `ERROR_SUCCESS` | **Registry 模块未导出**，需自定义常量（`WIN32_ERROR.0 == 0`） |
| `IInitializeWithFile` | 在 `Win32::UI::Shell::PropertiesSystem`，**不在** `Win32::UI::Shell` |
| `IInitializeWithItem` | 在 `Win32::UI::Shell`（与上面不同模块！） |
| `GUID::from_u128` | 返回 `GUID`，**不是** `Option<GUID>` |
| feature 门控 | 需显式加 `Win32_System_Registry`、`Win32_System_LibraryLoader`、`Win32_System_Console`、`Win32_UI_Shell_PropertiesSystem` |

**教训**：`--diag` 的 JSON 输出模式一开始不生效、`--json --diag` 会走成预览模式，
原因都是标志位只检查了 `args[0]`；已改为**顺序无关**判断。

---

## 这个程序已记录的数据

| 指标 | 实测值 | 用途 |
| --- | --- | --- |
| `--diag` 是否解析出处理器 | ✅ 是（`handler-found`） | 路线 A 在本机可用 |
| 处理器由谁提供 | WPS文字 预览器 `{0C7FEF07-…}` | 确认是 WPS |
| **`DoPreview` 端到端耗时** | **77 ms（热）/ 2554 ms（冷）** | 超时阈值可设 3–5 秒 |
| CLSID 位数 | **32 位**（仅 `WOW6432Node`） | **决定了 S1 宿进程必须编成 32 位** |

### 决策结论

耗时远低于 1.5 秒阈值、激活成功 → **按计划进 S1（宿进程化）**，且宿进程**编 32 位**用 `INPROC_SERVER` 直接载入，避免 DCOM 代理。

---

## 下一步

- **S1**：拆出 **32 位** `sfm-preview-host.exe`，加 IPC、看门狗、`Unload`
- **S2**：渲染器注册表抽象（**可独立成 PR**，不含 Windows 专有代码）
- **S3**：接 Vue 信息面板（量坐标、loading 态）
- **S4**：快速查看 + 降级链 + 设置项 + 诊断面板
