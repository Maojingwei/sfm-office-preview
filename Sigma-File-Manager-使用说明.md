# Sigma File Manager 使用说明

> 项目地址：<https://github.com/aleksey-hoffman/sigma-file-manager>
> 作者：Aleksey Hoffman　|　许可：GNU GPLv3 or later
> 本文基于 v2.2.0（v2 已从 beta 转为稳定版并成为主分支）、并保留 v1.x 的行为差异说明。
> 说明：撰写时本机网络无法直连 GitHub，内容来自项目 README、Release Notes、CHANGELOG 镜像与官方讨论区整理。

---

## 1. 项目是什么

Sigma File Manager（简称 SFM，可执行名 `sfm`）是一款**免费、开源、跨平台的现代文件管理器**，用来替代 Windows 资源管理器 / Linux 文件管理器。

- **定位**：不只是"文件浏览器"，而是以"智能搜索 + 标签页/分屏 + 扩展市场"为核心的生产力工具。
- **技术栈**：
  - **v1.x**：Electron + Vue 2 + Vuetify（当前 v1 分支**开发已暂停**，仅维护）。
  - **v2.x**：完全重写，**Rust（`src-tauri`，Tauri）+ Vue 3 + TypeScript**，抛弃 Electron。启动更快、体积更小（安装包 32 MB → 12 MB）、内存占用更低。
- **支持平台**：64 位 Windows、Linux 为主；macOS 代码支持但**不提供官方构建**，需自行编译。
- **项目状态**：约 6.5K star / 279 fork、1.39K+ 次提交，迭代非常活跃（2026 年内持续发布 2.1.0、2.2.0 等版本）。

### 一句话选型建议

| 你的需求 | 是否推荐 |
| --- | --- |
| 想要"秒搜整个硬盘 + 标签页 + 双栏"的现代替代品 | ✅ 强烈推荐 |
| 想用扩展市场扩展功能（下载视频、格式转换等） | ✅ 推荐 |
| 需要局域网把文件分享给手机/电视浏览器 | ✅ 推荐 |
| 生产环境重度依赖 SMB/NFS/SSHFS 挂载 | ⚠️ 该功能仍处 Alpha |
| 主力系统是 macOS | ⚠️ 无官方构建，需自行编译 |
| 机器内存 ≤ 4 GB | ⚠️ 不推荐（预览/视频仍可能占用较高内存） |

---

## 2. 核心功能总览

| 功能 | 说明 |
| --- | --- |
| **智能全局搜索** | 约 2 秒扫描 1 TB 硬盘；带纠错能力，可容忍拼写错误、大小写、词序颠倒、缺词、缺符号、缺扩展名 |
| **标签页（Tabs）** | 同时打开多个目录，一键/快捷键切换 |
| **分屏视图（Split view）** | 任一标签页可拆成两个面板，独立导航、互相拖拽传文件；含 `Split` 与 `Linked` 两种模式 |
| **扩展与市场** | 内置市场或本地文件夹安装扩展；扩展可注册命令、页面、快捷键、设置项、右键菜单、主题 |
| **设为默认文件管理器** | Windows 上可接管"文件资源管理器"图标、`Ctrl+E`、"在文件夹中显示"、浏览器下载完成提示、`start {path}` 等 |
| **局域网分享** | 浏览器访问 / 二维码 / FTP 两种模式（Stream 流式浏览、FTP 可上传下载） |
| **网络位置（Alpha）** | 连接 SSHFS、NFS、SMB、CIFS |
| **地址栏** | 键盘导航、路径自动补全、边打字边打开目录、一键跳到父目录 |
| **项目过滤器** | 用 glob 模式 + 属性前缀（大小、日期、MIME 等）过滤大目录 |
| **智能拖放** | 本地移动/复制更顺手；支持从浏览器拖 URL 下载、跨应用系统剪贴板 |
| **快捷键与命令面板** | 绝大多数操作可键盘完成；命令面板统一执行应用与扩展命令 |
| **首页横幅与视觉特效** | 自定义图片/视频/内置艺术画、透明度、混合模式、亮度对比度 |
| **仪表盘与快速访问** | 收藏、标签项、最常访问、历史记录集中一处；侧栏 Dashboard 悬浮即出快速面板 |
| **标签（Tags）** | 给文件/文件夹打标签，可改名、改色、删除 |
| **ZIP 归档** | 应用内压缩/解压 `.zip`，支持**带密码的加密 ZIP** 与非 UTF-8 文件名编码选择 |
| **最近编辑 / 链接管理** | 创建符号链接、快捷方式、硬链接、junction；可显示链接目标与状态 |
| **快速查看（Quick View）** | 按 `Space` 预览图片、视频、音频、PDF、文本，可在预览窗内翻页，不打开外部程序 |
| **WSL 驱动器** | Windows 上自动识别 WSL 发行版并作为驱动器浏览 |
| **应用内更新** | 更新通知里直接下载安装 |
| **多语言** | 覆盖主流语言，含中文、日语、越南语、印地语、乌尔都语、希伯来语（RTL） |

---

## 3. 安装

### 3.1 Windows

| 方式 | 命令 / 说明 |
| --- | --- |
| **winget（推荐）** | `winget install sfm` |
| **Microsoft Store** | 搜索 "Sigma File Manager"；**最省心，无杀毒告警** |
| **绿色安装包 .exe** | 未签名，可能出现 SmartScreen 安全警告：点 **"更多信息" → "仍要运行"** |

> 建议：普通用户优先 Microsoft Store 或 winget；只有需要便携部署时才用 `.exe`。

### 3.2 Linux

| 方式 | 命令 |
| --- | --- |
| **Flatpak** | `flatpak install ./Sigma-File-Manager-*-linux.flatpak` |
| **Snap** | `sudo snap install sigma-file-manager --edge` |
| **AppImage（实验性）** | 下载后 `chmod +x` 直接运行；建议配合 [AppImageLauncher](https://github.com/TheAssassin/AppImageLauncher) 集成到启动器/任务栏 |

> Linux 构建的开发投入少于 Windows，稳定性略差，遇到问题属预期范围。

### 3.3 macOS

官方**不提供构建产物**，需自行从源码编译（见第 12 节）。

### 3.4 环境要求

- 操作系统：64 位 Windows / Linux（macOS 需自编译）
- 内存：v1 时代最低约 100 MB、平均约 400 MB；v2 更轻量，但**播放视频预览、首页视频背景**仍会明显推高内存，低配机建议把背景设为静态图
- 磁盘：安装占用约 12 MB（v2 安装包）级别，另加扩展与缓存

---

## 4. 界面结构与上手路径

```
┌──────────────────────────────────────────────────────────┐
│ 标题栏 / 工具栏：导航按钮 · 地址栏 · 视图切换 · 状态中心   │
├────────────┬─────────────────────────────────────────────┤
│ 侧边栏     │  标签页栏（可拖拽、可关闭、可恢复）           │
│ · Home     │ ┌───────────────────┬─────────────────────┐ │
│ · 导航器   │ │ 面板 1（文件列表/网格）│ 面板 2（分屏时）  │ │
│ · Dashboard│ │                   │                   │ │
│ · 设置     │ └───────────────────┴─────────────────────┘ │
│ · 扩展     │  状态栏：当前面板指示 · 选中数量 · 操作进度   │
└────────────┴─────────────────────────────────────────────┘
                          （右侧可展开信息面板 Info panel）
```

**五个主页面**（`Alt+1` ~ `Alt+5` 直接切换）：**Home / Navigator（导航器）/ Dashboard / Settings / Extensions**。

### 新用户 10 分钟上手清单

1. 打开 **Navigator**，用地址栏或侧栏驱动器进入常用目录（如 `D:\Projects`）。
2. 右键该目录 → **Add to favorites（加入收藏）**，以后从 Dashboard 秒进。
3. 按 `Ctrl+T` 多开几个标签页：一个放项目、一个放下载、一个放素材。
4. 在任一标签页按 `Ctrl+S` 打开**分屏**，左右拖动即完成移动/复制。
5. 选中任意文件按 `Space` 试一次**快速查看**，再用方向键在预览里翻文件。
6. 按 `Ctrl+F` 在当前目录**快速搜索**，或打开**全局搜索**找整机文件。
7. 给重要目录打**标签**（Tag），并考虑开启**文件保护**防误删。
8. 进入 **Settings → General → Language** 切到中文；在 **Shortcuts** 里把全局唤醒键改成自己习惯的组合。
9. 进 **Extensions** 逛一次市场，装 1~2 个刚需扩展（如视频下载、图片压缩）。
10. （可选）**Settings → Experimental** 里把 SFM 设为**默认文件管理器**。

---

## 5. 文件与目录操作

### 5.1 选择

| 操作 | 方式 |
| --- | --- |
| 单选 | 左键点击 |
| 多选（连续） | 按住 `Shift` 点击，或方向键 + `Shift` |
| 多选（离散） | 按住 `Ctrl` 点击 |
| **框选（Box selection）** | 在空白处拖拽出选择框；`Ctrl`/`Shift` 追加到当前选择，`Alt` 反选 |
| 全选 | `Ctrl+A` |
| 右键空白处 | 清除当前选择并打开"当前目录"菜单 |

> 框选需在 `Settings > General > File view > Enable box selection` 打开。若列表/网格太密不好起手，同时打开 `Increase file view gaps` 增加内边距与网格间距。

### 5.2 基础操作

复制 `Ctrl+C` · 剪切 `Ctrl+X` · 粘贴 `Ctrl+V` · 删除 · 重命名 · 新建文件 `Ctrl+Shift+M` · 新建目录 `Ctrl+Shift+N` · 复制路径 (Copy path) · 打开方式 · 在终端打开（`Alt+T`，作用于**选中项**所在目录）

- **复制当前目录路径**：`Ctrl+Shift+C`
- **从剪贴板打开路径**：`Ctrl+Shift+V`（若剪贴板是合法路径，直接跳转）
- **刷新当前目录**：`F5`
- **重命名后的路径**：被关闭又恢复的标签页会保留重命名结果；已删除的路径会重定向到 Home

### 5.3 系统剪贴板互通（v2.2 起）

- 在 SFM 里 `Ctrl+C` 的文件，可粘贴进文件资源管理器等外部程序；反之亦然。
- 从浏览器复制的**图片**可直接 `Ctrl+V` 粘贴进当前文件夹。
- 粘贴冲突时弹窗选择 **Rename** 或 **Merge**，单个文件可选 **Replace / Skip / Keep both / Apply to all**。
- 可在 `Settings > UI appearance > Clipboard` 关闭工具栏预览（隐藏后 `Ctrl+V` 仍可用）。
- **单次粘贴**：复制的内容只能粘贴一次，避免误重复粘贴。

### 5.4 列表视图列

- 列宽可**拖动调整**，列顺序与显示可在表头 `Columns` 弹出面板中管理，支持 `Fill available width` 与 `Set minimum widths`。
- 可选列包括：**Created（创建时间）、Tags、Kind、Links、Link target、Link status** 等，可直接在列内添加/编辑标签。

### 5.5 归档

- **压缩**：多选文件/目录 → Compress → 生成 `.zip`
- **解压**：对 `.zip` → Extract（解到当前目录或指定文件夹）
- 加密 ZIP 会提示输入密码；文件名乱码时在 **Archive extraction options** 里选择编码（优先自动检测，另提供分区编码回退项）
- 解压会保留 Unix 文件权限位

### 5.6 链接（Links）

右键 → `Create link` 可创建**符号链接 / 快捷方式 / 硬链接 / junction**。

- 目录快捷方式与符号链接目录：导航到其目标
- 其他链接目标：用默认程序打开
- 需要排查时可开启 `Kind / Links / Link target / Link status` 列（状态为 `Valid` / `Broken` / `Unknown` / `Unsupported`）

### 5.7 拖放

- 本地正常拖拽移动/复制；拖到**标签页上**可移动到该标签页目录；悬停标签页可先切换过去。
- 支持从浏览器把链接拖进应用直接下载（v1 还有配合 yt-dlp 的视频下载能力，v2 该能力下放到**扩展**）。
- 通过 `Alt+Tab` 切换窗口也能把文件"拖出去"到别的应用。
- 拖入网络映射驱动器的外拖问题已在 v2.1 修复。

---

## 6. 搜索

SFM 有三种"找东西"的方式，各有分工。

### 6.1 快速搜索（当前目录，Quick search）

- **被动模式**：直接开始敲字母数字键即自动激活并过滤当前面板，不抢焦点、不阻断导航。
- **主动模式**：`Ctrl+F` 激活并聚焦输入框，可精细编辑查询串。
- 首个匹配项会自动选中，`Escape` 关闭。
- 在**非拉丁键盘布局**下输入也已修复可用性。

### 6.2 全局搜索（智能搜索，Global search）

- 索引整机文件，`1 TB` 级别驱动器约 2 秒出结果；增量索引持续后台维护。
- **容错**：拼写错误、大小写、词序、缺词、缺符号、缺扩展名都能命中。
- 结果支持键盘导航、回车/点击打开；空状态会给出"打开设置"入口。

### 6.3 项目过滤器 / 搜索语法（属性前缀）

用 `属性: 值` 的形式按任意属性筛选，值为**文件属性**（名称、大小、条目数、修改时间、创建时间、访问时间、路径、MIME 类型）：

```
name: report                    按名称
size: >=2mb                     大小比较（>= <= > <）
size: 1mb..10mb                 大小区间
modified: today                 修改时间（today / yesterday / 具体日期）
created: today                  创建时间
mime: image                     按 MIME 类型
path: D:/projects               按路径
```

- 也支持 **glob 模式**（如 `*.png`、`report-??.docx`）过滤当前目录。
- 属性前缀与 glob 可组合使用，适合在上万文件的大目录里迅速收窄范围。

---

## 7. 标签页、分屏与导航

### 7.1 标签页

- `Ctrl+T` 新建、`Ctrl+W` 关闭、`Ctrl+Tab` / `Ctrl+Shift+Tab` 切换、`Ctrl+Shift+T` **恢复最近关闭的标签组**（恢复到原位置、保留重命名、已删除路径回落到 Home）。
- **中键点击**目录可在新标签页打开；新标签会自动滚动进视野。
- 标签菜单有 **Close all duplicates**，一次清掉工作区里所有重复路径的标签页。
- 标签页是**拖放目标**：把文件拖到别的标签页即移动/复制过去。

### 7.2 分屏视图

- `Ctrl+S` 开关分屏；模式在导航器选项菜单 `Split view mode` 里选择：
  - **Split（独立）**：两个面板各自导航，适合"边看边搬"。
  - **Linked（联动）**：左面板点目录，右面板即显示其内容，接近传统"双栏/列式"工作流。
- 状态栏会明确标出**当前活动面板**。
- 窄窗口下分屏地址栏会自动换行到第二行。

### 7.3 导航方式

- **地址栏**：键盘可用、路径自动补全、边打字边打开目录；从驱动器根目录上跳或从地址栏打开 **Locations**（根位置列表，含驱动器与虚拟位置）。
- **地址编辑器**：可作为更强的"路径启动器"——既能开目录也能开文件；有"常用路径"模式；建议项覆盖目录条目、精确匹配、最近路径、标签项、用户文件夹、系统驱动器；支持后退/前进/上跳/在父目录中显示。
- **历史导航**：`Alt+←` / `Alt+→`，`Alt+↑` 到父目录；**鼠标侧键 4/5** 同样可前进后退。
- 侧边栏点驱动器 = 在当前标签页打开；`Locations` 可加收藏、打标签，尤其方便在分屏两栏间切驱动器。

---

## 8. 快速查看与信息面板

### 8.1 快速查看（Quick View）

- 选中文件按 `Space` 打开预览窗，支持**图片、视频、音频、PDF、常见纯文本**。
- 预览窗内可直接**切换到同目录的其它文件**，无需关闭重开。
- 文本预览支持**编码检测、行内编辑、Markdown 渲染**。
- 媒体预览使用生成的缩略图 + 虚拟列表，大目录下更省内存。
- 可在 `Settings > General > Performance` 开启 **Keep Quick View window in memory**（秒开，但常驻约 200 MB 内存）。

### 8.2 信息面板（Info panel）

- 展示选中项/当前目录的完整元信息，并可预览**所有 Quick View 支持的类型**（不只图片视频）：图片用缩略图、音视频带原生控件、PDF 内联渲染、文本给安全长度内的解码预览，不支持的类型用图标占位。
- 面板宽度、预览与详情的分割比例均可拖动调整；`Dynamic info panel size` 可让其自适应。
- 其它开关：`Show full-size image in info panel preview`、`Mute video preview by default`、`Automatically play video previews`。

---

## 9. 组织与安全：收藏 / 标签 / 仪表盘 / 文件保护

- **收藏（Favorites）**：右键目录 → 加入收藏，出现在 Dashboard 与侧栏快速访问面板。
- **标签（Tags）**：给文件/目录打标签；可**改名、改色、删除**；列表视图可显示 Tags 列并就地编辑；Dashboard 有独立"已打标签"区块。
- **Dashboard / 快速访问**：侧栏 `Dashboard` 按钮悬浮即弹出面板，显示收藏与标签项。面板里都是**真实目录条目**——可拖入拖出、右键菜单、执行所有标准文件操作。可在 `Settings > UI appearance > Open quick access panel on hover` 关闭悬浮行为。
- **Home 页**：可增删自定义用户目录；卡片有完整右键菜单（与导航器一致）；`Show home media banner` 可隐藏首页横幅。
- **文件保护（File protection，v1 特性）**：保护文件/目录/笔记不被修改、重命名、移动、删除——用于防误删关键数据。
- **历史 Time Line**：记录最近访问/操作，便于回溯。（v1 的 Notes 笔记编辑器属 v1 功能集，v2 以扩展形态延续类似能力。）

---

### 8.3 预览支持的格式范围（重要）

很多人把"信息面板（侧边栏）"和"快速查看"当成两套能力，其实二者支持范围基本一致：

| | 位置 | 触发方式 | 官方明确列出的能力 |
| --- | --- | --- | --- |
| **信息面板 Info panel** | 导航器右侧 | 选中文件即显示 | 图片（用生成的缩略图）、视频/音频（原生控件）、PDF（内联渲染）、文本（安全长度内的解码预览）；**不支持的文件与文件夹一律显示图标占位** |
| **快速查看 Quick View** | 独立预览窗 | 按 `Space` | 图片、视频、音频、PDF、常见纯文本；可在窗内翻页 |

**✅ 支持预览**：常见图片格式（含 GIF / APNG / WebP，用缩略图）· 视频 · 音频 · **PDF** · 纯文本（编码自动检测 + 行内编辑 + Markdown 渲染）

**❌ 不支持预览（显示图标占位）**：**Word（.docx）** · **Excel（.xlsx）** · PowerPoint（.pptx）· 其它二进制文档格式

官方对不支持类型的策略原话是"keep simple icon placeholders"。`.docx` / `.xlsx` 本质是 ZIP 包 + XML，要渲染需引入完整文档引擎，目前未实现。（v1 曾宣传"200+ 格式预览"，那个数字主要来自图片格式的广度，不含 Office 文档。）

**绕过办法**：

1. **在 Extensions 市场搜索**是否有文档预览类扩展——扩展可调用 ffmpeg / deno / node / 7z 等二进制，理论上有实现空间，但**官方未提供现成的 Office 预览扩展**。
2. **提功能请求**：到 <https://github.com/aleksey-hoffman/sigma-file-manager/issues> 新建 Feature request（如 "Preview support for .docx/.xlsx in info panel"）。
3. **换中介格式**：纯数据表格另存为 `.csv` 后即可当文本预览，还能用 `mime:` 前缀批量筛出；文档可转 PDF 或 Markdown。

> 排查提示：若**图片**也不显示预览，先检查 `Settings > UI appearance > Info panel > Show full-size image in info panel preview` 是否关闭、面板是否被折叠。若**只有** Office 类文件是图标，那属正常行为，不是设置问题。

> **想自己动手补这个能力？** 见配套文档：[SFM-复用系统预览处理器-可行性方案.md](SFM-复用系统预览处理器-可行性方案.md)——利用 WPS/Office 已注册的系统预览处理器（`IPreviewHandler`）实现 Office 预览，主程序体积零增长。

---
## 10. 局域网分享（LAN Sharing）

入口：导航器工具栏按钮，或任意文件/目录的右键菜单。分享激活后会显示**二维码**与**可访问 URL**。两种模式：

| 模式 | 用途 |
| --- | --- |
| **Stream** | 用**任意带现代浏览器的设备**（手机、平板、智能电视、笔记本、虚拟机）流式浏览/播放分享的文件与目录，无需在对方装任何软件 |
| **FTP** | 以 FTP 协议暴露目录，供其它应用直接访问；**双向**——既可下载也可从此设备上传 |

**典型用法**：把电影目录 Stream 分享出去 → 手机扫二维码 → 直接在电视/手机上播放，全程不用插线、不用装 App。

---

## 11. 扩展系统与市场

### 11.1 能力

- **安装渠道**：内置**市场**浏览/安装/管理，或从**本地文件夹**安装。
- **扩展可贡献**：
  - 命令（并出现在**命令面板**里）
  - 独立整页 UI（含新的 **list-detail** 可搜索列表 + 详情面板模式）
  - 本地与全局快捷键
  - 右键菜单项
  - 设置项
  - **应用配色主题**（出现在主题选择器）
  - **图标主题**（文件夹与文件图标主题可分开选择，见 `Settings > UI appearance > Icon Theme`；可按扩展名、文件名、目录名、展开状态定义）
  - **本地化翻译**（可提供多语言）
- **依赖与二进制**：扩展可使用 ffmpeg、deno、node、yt-dlp、7z 等任意二进制；在 `Extensions > Dependencies` 配置自动安装或手动指定本地二进制。
- **版本化**：可安装不同版本，可开启自动更新。
- **权限与沙箱**：扩展运行在隔离的 ESM 沙箱中，权限粒度化——例如 HTTP 请求仅限 manifest 声明的白名单主机，剪贴板读写、视图控制等需显式授权。
- **注意**：官方列出的扩展贡献点中**不包含"工具栏按钮"**；因此想用扩展补一个"书签栏"在 v2.2.0 上暂无明确可行的接入点（详见第 18 节）。

### 11.2 官方点名的扩展能力举例

- **视频下载**：支持 1000+ 站点（YouTube、Twitch 等）
- **媒体格式转换**（基于 ffmpeg）
- **图片体积优化 / 压缩**
- 主题与图标包

### 11.3 安全提示

安装第三方扩展前留意其申请的权限（剪贴板、HTTP 主机、二进制执行、视图控制）。只装可信来源；不使用时可禁用而非仅关闭页面。

---

## 12. 从源码构建（开发者）

### 12.1 前置环境

- **Rust** 工具链（v2 为 Tauri/Rust 后端，`src-tauri/`）
- **Node.js + pnpm**（Vue 3 + TypeScript 前端）
- **Tauri 平台依赖**：Windows 需 WebView2 与 MSVC 构建工具；Linux 需 WebKitGTK 等系统库
- Git

### 12.2 通用步骤（Tauri 项目惯例）

```bash
git clone https://github.com/aleksey-hoffman/sigma-file-manager
cd sigma-file-manager
pnpm install
pnpm tauri dev      # 开发运行
pnpm tauri build    # 打包当前平台安装包
```

> 具体脚本名以仓库 `package.json` 与 [CONTRIBUTING.md](https://github.com/aleksey-hoffman/sigma-file-manager/blob/main/CONTRIBUTING.md) 为准（项目在快速演进，脚本可能调整）。
> 贡献流程：**先开 Issue 或 Discussion 讨论改动，再提 PR**。

---

## 13. 设置项导览

设置按类别组织，重点项如下（括号内为路径）：

**General**

- 语言（`General > Language`）、日期时间格式与"相对时间(5 min ago)"（`General > Date / time`）
- 登录时自动启动（`General > Startup behavior`）
- 框选开关、增大列表/网格间距（`General > File view`）
- 性能：Quick View / Print 窗口常驻内存（各约 200 MB）（`General > Performance`）

**UI appearance**

- 首页媒体横幅显示/隐藏、Tooltip 延迟
- 样式：对话框背景模糊强度、界面**亮度/对比度**滤镜
- 视觉特效：覆盖层媒体亮度、`Mix Blend Mode`、空闲/最小化时暂停背景视频
- 信息面板：动态尺寸、全尺寸图片预览、默认静音、自动播放
- 剪贴板工具栏：外部图片 / 外部路径是否显示
- 图标主题、主题选择、快速访问面板悬浮开启

**Tabs**

- 活动标签文字加粗（`Tabs > Tab appearance > Bold active tab text`）

**Shortcuts**

- 多绑定（一个动作可绑多个快捷键）、解除绑定、**冲突时可直接替换**、列表右键菜单管理
- 支持**全局（系统级）快捷键**，例如"显示/隐藏应用"

**Experimental**

- **设为默认文件管理器（Windows）**：开启后接管资源管理器图标、`Ctrl+E`、"在文件夹中显示"、浏览器下载完成提示、`start {path}` / `code {path}` 等终端命令；回收站、控制面板这类深度集成的系统视图仍交回原生资源管理器。关闭时会安全还原系统原值。

---

## 14. 快捷键速查

> 以下为官方 Release Notes 中明确列出的快捷键；应用内 **Settings → Shortcuts** 是最权威、可自定义的完整清单。

### 导航与页面

| 快捷键 | 功能 |
| --- | --- |
| `Alt+1` ~ `Alt+5` | 切换 Home / Navigator / Dashboard / Settings / Extensions |
| `Alt+←` / `Alt+→` | 历史后退 / 前进 |
| `Alt+↑` | 到父目录 |
| 鼠标侧键 4 / 5 | 历史后退 / 前进 |
| `F5` | 刷新当前目录 |
| `F11` | 全屏切换 |
| `Ctrl+=` / `Ctrl+-` | 放大 / 缩小界面 |

### 标签页与视图

| 快捷键 | 功能 |
| --- | --- |
| `Ctrl+T` | 新建标签页 |
| `Ctrl+W` | 关闭标签页 |
| `Ctrl+Shift+T` | 恢复最近关闭的标签组 |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | 下一个 / 上一个标签页 |
| `Ctrl+S` | 开关分屏视图 |

### 文件操作

| 快捷键 | 功能 |
| --- | --- |
| `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | 复制 / 剪切 / 粘贴 |
| `Ctrl+Shift+M` | 新建文件 |
| `Ctrl+Shift+N` | 新建目录 |
| `Ctrl+Shift+C` | 复制当前目录路径 |
| `Ctrl+Shift+V` | 打开剪贴板中的路径 |
| `Alt+Enter` | 打开 Windows 原生"属性"窗口 |
| `Alt` + 双击 | 同上（原生属性） |
| `Alt+T` | 在终端打开（选中项目录） |

### 搜索与预览

| 快捷键 | 功能 |
| --- | --- |
| `Ctrl+F` | 快速搜索（主动模式） |
| 直接输入字母数字 | 快速搜索（被动模式，自动激活） |
| `Space` | 快速查看预览 |
| `Escape` | 关闭全局搜索 / 打印视图 / 弹层 |
| `Ctrl+O` | 打印选中文件（图片/PDF/文本） |

> 提示：`Delete`、`F2`、`Enter` 等常规键位按惯例工作，并可在 `Settings → Shortcuts` 中改写或另绑。

---

## 15. 实用工作流示例

**A. 整理下载目录**

1. 打开下载目录 → `Ctrl+F` 输入 `mime: image` 挑出所有图片 → 剪切粘贴到 `Pictures`。
2. 再筛 `size: >=100mb` 找出大文件单独归档。
3. 多选 → Compress 打成 `.zip`，原文件删除后放进归档目录。

**B. 双栏搬素材（Linked 模式）**

1. `Ctrl+S` 开分屏，模式选 **Linked**。
2. 左栏点到项目目录，右栏自动跟随显示其内容。
3. 从右栏把素材拖进左栏目标文件夹（或反向），完成整理。

**C. 手机取电脑上的电影**

1. 右键电影目录 → LAN sharing → **Stream**。
2. 手机扫二维码，在浏览器里直接播放；或在电视浏览器输入分享 URL。
3. 需要对方上传时改用 **FTP** 模式。

**D. 接管资源管理器**

1. `Settings → Experimental` → 开启默认文件管理器。
2. 之后 Win 资源管理器图标、`Ctrl+E`、"在文件夹中显示"、下载完成提示都会走 SFM。
3. 不习惯随时关闭，系统原注册表值会被安全还原。

**E. 给关键目录上双保险**

1. 右键重要目录 → 打标签（如 `work`）→ 加入 Favorites。
2. 打开 **File protection**，防止误改/误删。
3. 再在 Dashboard 里一眼找到它。

---

## 16. 常见问题与注意事项

**Q：安装时被杀毒/SmartScreen 拦截？**
`.exe` 安装包未签名，属正常现象：点"更多信息 → 仍要运行"。最省事是用 **Microsoft Store** 或 **winget install sfm**。

**Q：内存占用偏高？**
主因通常是**视频预览**与**首页视频背景**。改用静态图片作为首页横幅和视觉特效底图；关闭 Quick View / Print 窗口常驻；避免在 4 GB 内存及以下机器上重度使用。

**Q：SMB/NFS/SSHFS 不稳定？**
网络位置功能官方标注为 **Alpha 阶段**，出现问题是预期内的。关键业务建议仍用系统挂载 + 本地路径访问。

**Q：多文件复制/删除时界面卡住？**
v2 已修复大数量操作冻结 UI 的问题，并在**状态中心**显示进度、支持并行取消。若仍在旧版本，请升级。

**Q：Linux 上和 Windows 体验不一致？**
作者自述主要面向 Windows 开发，Linux 构建投入较少、问题更多。

**Q：v1 的功能（Notes 笔记、Workspaces 工作区、高级下载器、7z 归档）在 v2 还有吗？**
v2 是重写版本，功能集有取舍：部分能力（如视频下载、格式转换）转为**扩展**提供；跨平台归档目前聚焦 `.zip`。若你高度依赖 v1 特有功能，可继续用 v1，但注意 **v1 开发已暂停**。

**Q：数据安全？**
应用本身以 GNU GPLv3 发布，不会主动删除或破坏文件。但任何文件管理器都应遵循基本习惯：**重要数据保持独立备份**。

---

## 18. 能否像浏览器那样把收藏夹做成工具栏？

**结论：v2.2.0 没有"浏览器书签栏"式的常驻收藏工具栏。**

官方文档与各版本 Release Notes 中，收藏（Favorites）的展示位置只有三处：**Dashboard 页面**、**侧边栏 Dashboard 悬浮快速访问面板**、**地址栏的 Locations 列表**。没有任何"工具栏显示收藏文件夹"的开关或设置项。

### 可用的替代方案（按接近程度排序）

| 替代方式 | 操作 | 接近程度 |
| --- | --- | --- |
| **快速访问悬浮面板** | 鼠标移到侧栏 `Dashboard` 按钮上即弹出收藏 + 标签项；面板内是真实目录条目，可拖拽、右键、执行全部文件操作 | ★★★★☆ 最接近"下拉式书签栏"，还支持拖拽 |
| **自定义 Home 页目录** | Home 页可**添加/移除用户目录**卡片，点一下即进入 | ★★★★☆ 常驻可见，最省事 |
| **收藏进侧边栏 / 地址栏 Locations** | `Locations` 支持加收藏、打标签，适合快速切驱动器 | ★★★☆☆ |
| **标签（Tags）+ Dashboard** | 给目录打标签，Dashboard 有独立"已打标签"区块 | ★★★☆☆ |
| **固定标签页** | 用 `Ctrl+T` 把常用目录各开一个标签页常驻 | ★★★☆☆ 浏览器"固定标签页"的同款思路 |
| **命令面板** | 通过命令面板快速调用 | ★★☆☆☆ |

建议组合：**Home 页放最常用的 5~8 个目录**（常驻可见）+ **Dashboard 悬浮面板放全部收藏**（随时下拉）+ **标签页放正在用的三五个目录**（零延迟切换）。

### 想真正要一条书签栏怎么办

1. **提功能请求**：到 <https://github.com/aleksey-hoffman/sigma-file-manager/issues> 新建 Feature request，标题可写 "Favorites/bookmarks toolbar in navigator"。项目迭代速度很快（一个版本周期内常加入多条 UI 类特性），这类"把已有数据换个位置展示"的需求实现成本不高，值得提。
2. **扩展方式暂不可行**：扩展可贡献的能力清单里**没有工具栏按钮**，只有命令、整页 UI、快捷键、右键菜单项、设置项、主题/图标主题、翻译。所以"自己写个扩展加书签栏"目前没有接入点。
3. **自建源码分支**：v2 为 Rust(Tauri) + Vue 3，前端改造成本可控，可在工具栏区域加一条收藏栏后自行编译（见第 12 节）。

---

## 19. 参考链接

- 仓库主页：<https://github.com/aleksey-hoffman/sigma-file-manager>
- 发布与变更日志：<https://github.com/aleksey-hoffman/sigma-file-manager/releases> · `CHANGELOG.md`
- 贡献指南：`CONTRIBUTING.md`
- 社区：Discord、Reddit [r/SigmaFileManager](https://www.reddit.com/r/SigmaFileManager)、[YouTube](https://www.youtube.com/@sigma-dev)、[X/Twitter](https://twitter.com/sigma__dev)、[Telegram](https://t.me/sigma_devs)
- 支持作者：[Patreon](https://patreon.com/sigma_file_manager)
- 版本说明来源：[v2.2.0 Release Notes](https://newreleases.io/project/github/aleksey-hoffman/sigma-file-manager/release/v2.2.0)、[v2.1.0 Release Notes](https://newreleases.io/project/github/aleksey-hoffman/sigma-file-manager/release/v2.1.0)、[v2.0.0-beta.3 Release Notes](https://newreleases.io/project/github/aleksey-hoffman/sigma-file-manager/release/v2.0.0-beta.3)
- v2 README 镜像：<https://mygit.top/repository/370679811>
- 历史版本（v1.7）功能说明：<https://explore.market.dev/ecosystems/electron/projects/sigma-file-manager>
- 配套文档：[SFM-复用系统预览处理器-可行性方案.md](SFM-复用系统预览处理器-可行性方案.md)（Office 预览的扩展方案设计）
- Windows 机制参考：[注册预览处理器](https://learn.microsoft.com/zh-cn/previous-versions//bb776868(v=vs.85))、[缩略图处理器](https://learn.microsoft.com/en-us/windows/win32/shell/thumbnail-providers)、[IPreviewHandler::SetWindow](https://learn.microsoft.com/ja-jp/windows/win32/api/shobjidl_core/nf-shobjidl_core-ipreviewhandler-setwindow)、[Tauri 侧车](https://v2.tauri.app/develop/sidecar/)
