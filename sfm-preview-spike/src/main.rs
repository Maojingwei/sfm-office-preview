//! S0 spike — 宿主化 Windows 已注册的预览处理器（IPreviewHandler）
//!
//! 目的：用最小代码验证「复用 WPS/Office 已注册的预览处理器」这条路在你机器上是否走得通。
//! 不做 UI 集成、不做进程隔离、不做降级 —— 只回答一个问题：
//!     DoPreview() 能不能把一个 .docx 真实地画出来？耗时多少？
//!
//! 用法：
//!     sfm-preview-spike.exe "C:\path\to\file.docx"
//!     sfm-preview-spike.exe --diag "C:\path\to\file.docx"     # 只打印探测结果，不开窗
//!
//! 依赖：Windows SDK（链接 shell32 / ole32，windows crate 会自动链接）

#![cfg(windows)]

use std::path::Path;
use std::time::Instant;

use windows::core::{Interface, PCWSTR, PWSTR, GUID};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetStockObject, UpdateWindow, WHITE_BRUSH};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, CLSCTX_LOCAL_SERVER,
    COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CLASSES_ROOT, HKEY_CURRENT_USER,
    HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ,
};
use windows::Win32::UI::Shell::PropertiesSystem::IInitializeWithFile;
use windows::Win32::UI::Shell::{
    AssocQueryStringW, IInitializeWithItem, IPreviewHandler, SHCreateItemFromParsingName,
    ASSOCF_NONE, ASSOCSTR, ASSOCSTR_PROGID, ASSOCSTR_SHELLEXTENSION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, GetMessageW, LoadCursorW,
    PostQuitMessage, RegisterClassW, ShowWindow, TranslateMessage, CREATESTRUCTW, CS_HREDRAW,
    CS_VREDRAW, CW_USEDEFAULT, IDC_ARROW, MSG, SW_SHOW, WM_CREATE, WM_DESTROY, WM_SIZE,
    WNDCLASSW, WS_CHILD, WS_EX_CLIENTEDGE, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};

/// 预览处理器的固定扩展点 GUID。
/// 注意：缩略图处理器是 {E357FCCD-A995-4576-B01F-234630154E96}，别混用。
const SHELLEX_PREVIEW_HANDLER: &str =
    "{8895B1C6-B41F-4C1C-A562-0D564250836F}";

/// 处理器是否已登记进全局表（用于体检）
const PREVIEW_HANDLERS_KEY: &str =
    r"SOFTWARE\Microsoft\Windows\CurrentVersion\PreviewHandlers";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("用法: sfm-preview-spike.exe [--diag] \"<文件路径>\"");
        std::process::exit(2);
    }

    let diag_only = args[0].eq_ignore_ascii_case("--diag");
    let json_mode = args.iter().any(|a| a.eq_ignore_ascii_case("--json"));
    let positional: Vec<String> = args
        .iter()
        .filter(|a| !a.starts_with("--"))
        .cloned()
        .collect();

    let path = if diag_only {
        match args.iter().skip(1).find(|a| !a.starts_with("--")) {
            Some(p) => p.clone(),
            None => {
                eprintln!("--diag 需要文件路径");
                std::process::exit(2);
            }
        }
    } else {
        match positional.first() {
            Some(p) => p.clone(),
            None => {
                eprintln!("用法: sfm-preview-spike.exe [--diag] [--json] \"<文件路径>\"");
                std::process::exit(2);
            }
        }
    };

    if !Path::new(&path).exists() {
        eprintln!("[!] 文件不存在: {path}");
        std::process::exit(2);
    }

    if diag_only {
        if json_mode {
            print_diag_json(&path);
        } else {
            report_diagnostics(&path);
        }
        return;
    }

    // COM：STA。预览处理器多数要求单线程套间。
    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
        if hr.is_err() {
            eprintln!("[!] CoInitializeEx 失败: {hr:?}");
            std::process::exit(1);
        }
    }

    report_diagnostics(&path);

    match run_window_and_preview(&path) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("\n[!] 预览失败: {e}");
            eprintln!("    参考上方的探测报告判断是「没注册」还是「注册了但调不通」。");
            std::process::exit(1);
        }
    }

    unsafe { CoUninitialize() };
}

/// 机器可读的探测结果，供 CI 解析（单行 JSON，便于 `Select-String '^{"'` 抓取）
fn print_diag_json(path: &str) {
    let ext = ext_of(path).unwrap_or_default();
    let shx_sub = format!(r"{ext}\ShellEx\{SHELLEX_PREVIEW_HANDLER}");

    let hkcu = reg_default_string_hkcu(&shx_sub).and_then(|s| parse_guid(&s));
    let hklm = reg_default_string_hklm_classes(&shx_sub).and_then(|s| parse_guid(&s));
    let progid = resolve_progid(path).ok().flatten().and_then(|p| {
        reg_default_string_hkcr(&format!(r"{p}\shellex\{SHELLEX_PREVIEW_HANDLER}"))
            .and_then(|s| parse_guid(&s))
    });

    let effective = hkcu.or(hklm).or(progid);
    let registered = match effective {
        Some(g) => reg_default_string(&format!(r"{PREVIEW_HANDLERS_KEY}\{{{g:?}}}")).is_some(),
        None => false,
    };

    let json = format!(
        r#"{{"ext":"{}","hkcu":"{}","hklm":"{}","progid":"{}","effective":"{}","registered":{},"verdict":"{}"}}"#,
        ext,
        guid_str(&hkcu),
        guid_str(&hklm),
        guid_str(&progid),
        guid_str(&effective),
        registered,
        if effective.is_some() {
            "handler-found"
        } else {
            "no-handler"
        }
    );
    println!("{json}");
}

fn guid_str(g: &Option<GUID>) -> String {
    match g {
        Some(g) => format!("{{{g:?}}}"),
        None => String::new(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 窗口 + 预览
// ─────────────────────────────────────────────────────────────────────────────

/// 创建顶层窗口 + 子窗口（子窗口就是交给处理器的"画布"），然后 DoPreview。
fn run_window_and_preview(path: &str) -> Result<(), String> {
    let hinst = unsafe { windows::Win32::System::LibraryLoader::GetModuleHandleW(None) }
        .map_err(|e| format!("GetModuleHandleW: {e:?}"))?;

    let class_name = wide("SfmPreviewSpikeClass");
    let title = wide(&format!("S0 spike — {path}"));

    unsafe {
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinst.into(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH(
                GetStockObject(WHITE_BRUSH).0,
            ),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };
        if RegisterClassW(&wc) == 0 {
            return Err("RegisterClassW 失败".into());
        }

        // 把文件路径塞进 CREATESTRUCTW.lpCreateParams，窗口创建后取回
        let param = Box::into_raw(Box::new(path.to_string()));

        let hwnd = CreateWindowExW(
            Default::default(),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(title.as_ptr()),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            900,
            700,
            None,
            None,
            hinst_instance(hinst),
            Some(param as *const _),
        )
        .map_err(|e| format!("CreateWindowExW: {e:?}"))?;

        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = UpdateWindow(hwnd);

        // 消息循环
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    Ok(())
}

/// 窗口过程：WM_SIZE 时重建/调整预览区域，WM_DESTROY 时收尾。
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // 用 GWLP_USERDATA 挂一个状态结构
    thread_local! {
        static STATE: std::cell::RefCell<Option<PreviewState>> =
            const { std::cell::RefCell::new(None) };
    }

    match msg {
        WM_CREATE => {
            let cs = lparam.0 as *const CREATESTRUCTW;
            let path = if cs.is_null() {
                String::new()
            } else {
                let raw = (*cs).lpCreateParams as *const String;
                if raw.is_null() {
                    String::new()
                } else {
                    // 取回所有权，避免泄漏
                    let boxed = Box::from_raw(raw as *mut String);
                    (*boxed).clone()
                }
            };

            // 注意：WM_CREATE 阶段窗口尺寸尚未确定，不在这里挂载预览。
            // 真正的挂载放在首次 WM_SIZE（那时客户区尺寸才知道）。
            let state = PreviewState {
                path,
                child: None,
                handler: None,
                last_error: None,
                start: None,
            };
            STATE.with(|s| *s.borrow_mut() = Some(state));
            LRESULT(0)
        }

        WM_SIZE => {
            // 让子窗口铺满客户区（留 20px 给顶部提示）
            let mut rc = RECT::default();
            let _ = GetClientRect(hwnd, &mut rc);
            let w = rc.right - rc.left;
            let h = rc.bottom - rc.top;
            let ph = (h - 24).max(1);

            // 先判断是否需要首次挂载（锁在块内释放，避免与下方借用冲突）
            let need_attach = STATE.with(|s| {
                s.borrow()
                    .as_ref()
                    .map(|st| st.child.is_none() && st.last_error.is_none())
                    .unwrap_or(false)
            });

            if need_attach {
                // 首次：此刻客户区尺寸才确定。在锁外调用，打日志/COM 调用都安全。
                let r = STATE.with(|s| {
                    let mut guard = s.borrow_mut();
                    let state = guard.as_mut().unwrap();
                    ensure_child_and_resize(hwnd, state, w, ph)
                });
                if let Err(e) = r {
                    eprintln!("\n[!] 挂载预览失败: {e}");
                    STATE.with(|s| {
                        if let Some(st) = s.borrow_mut().as_mut() {
                            st.last_error = Some(e);
                        }
                    });
                } else {
                    println!("\n[✓] DoPreview 已返回，窗口应已显示内容。");
                }
            } else {
                // 后续：随窗口缩放调整预览区域
                STATE.with(|s| {
                    let mut guard = s.borrow_mut();
                    if let Some(state) = guard.as_mut() {
                        if let Some(child) = state.child {
                            let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowPos(
                                child,
                                None,
                                0,
                                24,
                                w.max(1),
                                ph,
                                windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER,
                            );
                            if let Some(handler) = &state.handler {
                                let r = RECT {
                                    left: 0,
                                    top: 0,
                                    right: w.max(1),
                                    bottom: ph,
                                };
                                let _ = handler.SetRect(&r);
                            }
                        }
                    }
                });
            }
            LRESULT(0)
        }

        WM_DESTROY => {
            STATE.with(|s| {
                let mut guard = s.borrow_mut();
                if let Some(state) = guard.as_mut() {
                    // 必须 Unload：否则 WPS 进程驻留、文件句柄不释放
                    if let Some(handler) = &state.handler {
                        let _ = handler.Unload();
                        println!("[i] 已调用 IPreviewHandler::Unload()");
                    }
                    state.handler = None;
                }
            });
            PostQuitMessage(0);
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

struct PreviewState {
    path: String,
    child: Option<HWND>,
    handler: Option<IPreviewHandler>,
    last_error: Option<String>,
    start: Option<Instant>,
}

fn ensure_child_and_resize(
    parent: HWND,
    state: &mut PreviewState,
    w: i32,
    h: i32,
) -> Result<(), String> {
    let hinst = unsafe { windows::Win32::System::LibraryLoader::GetModuleHandleW(None) }
        .map_err(|e| format!("{e:?}"))?;
    let child_class = wide("STATIC");
    let empty = wide("");

    // 子窗口作为"画布"。注意：处理器会往这个 HWND 上自己画，
    // 我们不要对它自绘。
    let child = unsafe {
        CreateWindowExW(
            WS_EX_CLIENTEDGE,
            PCWSTR(child_class.as_ptr()),
            PCWSTR(empty.as_ptr()),
            WS_CHILD | WS_VISIBLE,
            0,
            24,
            w.max(1),
            h.max(1),
            parent,
            None,
            hinst_instance(hinst),
            None,
        )
    }
    .map_err(|e| format!("创建子窗口失败: {e:?}"))?;

    state.child = Some(child);

    // 解析 + 创建处理器
    let (clsid, via) = resolve_preview_clsid(&state.path)?
        .ok_or_else(|| "该类型没有任何注册路径（shellex 与 PersistentHandler 均为空）".to_string())?;
    println!("[i] CLSID 来源: {via}");

    let start = Instant::now();
    // 关键：用 LOCAL_SERVER。WPS 预览器是 32 位组件（仅注册于 WOW6432Node、只有 InprocHandler32），
    // 64 位宿主用 INPROC_SERVER 必然失败，必须让 DCOM 走代理宿主（如 SysWOW64\prevhost.exe）。
    // 同时保留 INPROC_SERVER 作为 64 位处理器的回退。
    let handler: IPreviewHandler = unsafe {
        CoCreateInstance(&clsid, None, CLSCTX_LOCAL_SERVER | CLSCTX_INPROC_SERVER)
    }
    .map_err(|e| {
        format!(
            "CoCreateInstance({clsid:?}) 失败: {e:?}\n\
             若为 REGDB_E_CLASSNOTREG：该 CLSID 在当前位数视图下不存在（32/64 位不匹配）。\n\
             若为 CO_E_SERVER_EXEC_FAILURE / 0x80080005：代理宿主启动失败。"
        )
    })?;

    // 初始化数据源：先试 IInitializeWithFile，失败再试 IInitializeWithItem
    let abs = absolute_path(&state.path);
    let wide_path = wide(&abs);
    let mut initialized = false;

    if let Ok(init_file) = handler.cast::<IInitializeWithFile>() {
        match unsafe { init_file.Initialize(PCWSTR(wide_path.as_ptr()), 0x0 /* STGM_READ */) } {
            Ok(()) => {
                println!("[i] 初始化方式: IInitializeWithFile");
                initialized = true;
            }
            Err(e) => eprintln!("[i] IInitializeWithFile 失败({e:?})，改试 IInitializeWithItem"),
        }
    }

    if !initialized {
        let item = unsafe {
            SHCreateItemFromParsingName::<_, _, windows::Win32::UI::Shell::IShellItem>(
                PCWSTR(wide_path.as_ptr()),
                None,
            )
        }
        .map_err(|e| format!("SHCreateItemFromParsingName 失败: {e:?}"))?;

        let init_item = handler
            .cast::<IInitializeWithItem>()
            .map_err(|e| format!("处理器不支持 IInitializeWithItem: {e:?}"))?;
        unsafe { init_item.Initialize(&item, 0x0) }
            .map_err(|e| format!("IInitializeWithItem::Initialize 失败: {e:?}"))?;
        println!("[i] 初始化方式: IInitializeWithItem");
    }

    let rect = RECT {
        left: 0,
        top: 0,
        right: w.max(1),
        bottom: h.max(1),
    };

    unsafe { handler.SetWindow(child, &rect) }
        .map_err(|e| format!("SetWindow 失败: {e:?}"))?;
    unsafe { handler.SetRect(&rect) }.map_err(|e| format!("SetRect 失败: {e:?}"))?;

    // DoPreview 是同步阻塞的 —— 这就是我们要测的那个数字
    unsafe { handler.DoPreview() }.map_err(|e| format!("DoPreview 失败: {e:?}"))?;

    let elapsed = start.elapsed();
    println!("[✓] 端到端耗时: {} ms", elapsed.as_millis());
    state.start = Some(start);
    state.handler = Some(handler);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// 注册表解析
// ─────────────────────────────────────────────────────────────────────────────

/// 取扩展名（小写，含点）
fn ext_of(path: &str) -> Option<String> {
    Path::new(path)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
}

/// 解析某扩展名注册的预览处理器 CLSID。
///
/// **注册点有三处，必须全部探测**（实测教训，逐轮修正得来）：
///   ① HKLM\SOFTWARE\Classes\<ext>\ShellEx\{8895B1C6-…}  ← 实测中 **WPS 用的就是这个**
///   ② HKCU\SOFTWARE\Classes\<ext>\ShellEx\{8895B1C6-…}  ← **优先级最高，会覆盖 HKLM**
///   ③ HKCR\<ext> 的 ProgID\shellex\{8895B1C6-…}        ← 传统位置（本机为空）
///
/// 注意：
/// - 生效值取 **HKCU > HKLM**。
/// - 本机 `HKLM\<ext>\ShellEx` 指向 `{84F66100-…}`，但该 CLSID 在注册表里**根本不存在**；
///   真正生效的是 HKCU 的 `{0C7FEF07-…}`。所以**不能只看 HKLM**。
/// - 该 CLSID 只在 `WOW6432Node` 下注册、且只有 `InprocHandler32`（无 `InprocServer32`），
///   意味着它是 **32 位、进程外/代理宿主** 形态（见 README「宿主策略」）。
fn resolve_preview_clsid(path: &str) -> Result<Option<(GUID, &'static str)>, String> {
    let ext = match ext_of(path) {
        Some(e) => e,
        None => return Ok(None),
    };

    // ① / ② 扩展名直属 ShellEx：HKCU 优先
    let shellex_sub = format!(r"{ext}\ShellEx\{SHELLEX_PREVIEW_HANDLER}");
    if let Some(s) = reg_default_string_hkcu(&shellex_sub) {
        if let Some(g) = parse_guid(s.trim()) {
            return Ok(Some((g, "HKCU\\<ext>\\ShellEx")));
        }
    }
    if let Some(s) = reg_default_string_hklm_classes(&shellex_sub) {
        if let Some(g) = parse_guid(s.trim()) {
            return Ok(Some((g, "HKLM\\Classes\\<ext>\\ShellEx")));
        }
    }

    // ③ 传统位置：ProgID\shellex
    if let Some(s) = assoc_query(&ext, ASSOCSTR_SHELLEXTENSION, SHELLEX_PREVIEW_HANDLER)? {
        if let Some(g) = parse_guid(s.trim()) {
            return Ok(Some((g, "ProgID\\shellex")));
        }
    }

    Ok(None)
}

/// 取文件关联的 ProgID
fn resolve_progid(path: &str) -> Result<Option<String>, String> {
    let ext = match ext_of(path) {
        Some(e) => e,
        None => return Ok(None),
    };
    assoc_query(&ext, ASSOCSTR_PROGID, "")
}

/// 通用 AssocQueryStringW 包装
fn assoc_query(assoc: &str, str_kind: ASSOCSTR, extra: &str) -> Result<Option<String>, String> {
    let assoc_w = wide(assoc);
    let extra_w = wide(extra);

    let mut len: u32 = 0;
    // 先问长度（预期返回 S_FALSE + 需要的长度）
    unsafe {
        let _ = AssocQueryStringW(
            ASSOCF_NONE,
            str_kind,
            PCWSTR(assoc_w.as_ptr()),
            PCWSTR(extra_w.as_ptr()),
            PWSTR::null(),
            &mut len,
        );
    }
    if len == 0 {
        return Ok(None);
    }

    let mut buf = vec![0u16; len as usize];
    let hr = unsafe {
        AssocQueryStringW(
            ASSOCF_NONE,
            str_kind,
            PCWSTR(assoc_w.as_ptr()),
            PCWSTR(extra_w.as_ptr()),
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
    };
    if hr.is_err() {
        return Ok(None);
    }

    let s = String::from_utf16_lossy(&buf[..len.saturating_sub(1) as usize]);
    Ok(Some(s))
}

/// 从 HKLM 下的某个子键读默认值
fn reg_default_string(subkey: &str) -> Option<String> {
    reg_default_string_at(HKEY_LOCAL_MACHINE, subkey)
}

/// 从 HKCR 下的某个子键读默认值（如 ".docx\PersistentHandler"）
fn reg_default_string_hkcr(subkey: &str) -> Option<String> {
    reg_default_string_at(HKEY_CLASSES_ROOT, subkey)
}

/// 从 HKCU\SOFTWARE\Classes 下读默认值（用户级关联，优先级最高）
fn reg_default_string_hkcu(subkey: &str) -> Option<String> {
    reg_default_string_at(HKEY_CURRENT_USER, &format!(r"SOFTWARE\Classes\{subkey}"))
}

/// 从 HKLM\SOFTWARE\Classes 下读默认值（机器级，64 位视图）
fn reg_default_string_hklm_classes(subkey: &str) -> Option<String> {
    reg_default_string_at(HKEY_LOCAL_MACHINE, &format!(r"SOFTWARE\Classes\{subkey}"))
}

/// 读默认值的通用实现
fn reg_default_string_at(root: HKEY, subkey: &str) -> Option<String> {
    let sub = wide(subkey);
    let mut hkey = HKEY::default();
    // windows 0.58: ulOptions 是裸 u32
    let rc = unsafe { RegOpenKeyExW(root, PCWSTR(sub.as_ptr()), 0, KEY_READ, &mut hkey) };
    if rc.0 != WIN32_ERROR_SUCCESS {
        return None;
    }

    let mut ty = REG_SZ;
    let mut len: u32 = 0;
    // 先问长度
    let rc = unsafe {
        RegQueryValueExW(
            hkey,
            PCWSTR::null(),
            None,
            Some(&mut ty),
            None,
            Some(&mut len),
        )
    };
    if rc.0 != WIN32_ERROR_SUCCESS || len == 0 {
        let _ = unsafe { RegCloseKey(hkey) };
        return None;
    }

    let mut buf = vec![0u8; len as usize];
    let rc = unsafe {
        RegQueryValueExW(
            hkey,
            PCWSTR::null(),
            None,
            Some(&mut ty),
            Some(buf.as_mut_ptr()),
            Some(&mut len),
        )
    };
    let _ = unsafe { RegCloseKey(hkey) };
    if rc.0 != WIN32_ERROR_SUCCESS {
        return None;
    }

    let u16s: Vec<u16> = buf
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&c| c != 0)
        .collect();
    Some(String::from_utf16_lossy(&u16s))
}

fn parse_guid(s: &str) -> Option<GUID> {
    let t = s.trim().trim_start_matches('{').trim_end_matches('}');
    let hex = t.replace('-', "");
    if hex.len() != 32 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    u128::from_str_radix(&hex, 16).ok().map(GUID::from_u128)
}

// ─────────────────────────────────────────────────────────────────────────────
// 诊断报告
// ─────────────────────────────────────────────────────────────────────────────

fn report_diagnostics(path: &str) {
    let ext = ext_of(path).unwrap_or_else(|| "<无扩展名>".into());

    println!("════════ 预览处理器探测报告 ════════");
    println!("文件      : {path}");
    println!("扩展名    : {ext}");
    println!(
        "文件存在  : {}",
        if Path::new(path).exists() { "是" } else { "否" }
    );

    let progid = match resolve_progid(path) {
        Ok(Some(p)) => {
            println!("ProgID    : {p}");
            Some(p)
        }
        Ok(None) => {
            println!("ProgID    : <未关联>");
            None
        }
        Err(e) => {
            println!("ProgID    : 查询失败 {e}");
            None
        }
    };

    // ── 三处注册点逐一探测
    let shx_sub = format!(r"{ext}\ShellEx\{SHELLEX_PREVIEW_HANDLER}");

    let hkcu_v = reg_default_string_hkcu(&shx_sub).and_then(|s| parse_guid(&s));
    let hklm_v = reg_default_string_hklm_classes(&shx_sub).and_then(|s| parse_guid(&s));

    let progid_v = progid.as_ref().and_then(|p| {
        reg_default_string_hkcr(&format!(r"{p}\shellex\{SHELLEX_PREVIEW_HANDLER}"))
            .and_then(|s| parse_guid(&s))
    });

    println!("① HKCU\\...\\{ext}\\ShellEx   : {}", fmt_guid(&hkcu_v));
    println!("② HKLM\\Classes\\{ext}\\ShellEx: {}", fmt_guid(&hklm_v));
    println!("③ ProgID\\shellex             : {}", fmt_guid(&progid_v));
    println!("   （生效优先级：① > ② > ③）");

    // ── 全局登记表
    println!(
        "全局 PreviewHandlers 表      : {}",
        if reg_default_string(&format!(r"{PREVIEW_HANDLERS_KEY}\{{{:?}}}", hkcu_v.or(hklm_v).unwrap_or_default())).is_some() {
            "已登记"
        } else {
            "未登记"
        }
    );

    // ── 选中的 CLSID
    let chosen = hkcu_v.or(hklm_v).or(progid_v);
    match chosen {
        Some(g) => {
            println!("\n── 生效 CLSID: {{{g:?}}}");

            // 64 位视图
            let k64 = format!(r"CLSID\{{{g:?}}}");
            // 32 位视图
            let k32 = format!(r"WOW6432Node\Classes\CLSID\{{{g:?}}}");

            for (label, key) in [("64位视图", k64.as_str()), ("32位视图", k32.as_str())] {
                let name = reg_default_string_hklm_classes(key);
                let impl32 = reg_default_string(&format!(r"{key}\InprocServer32"));
                let impl_local32 = reg_default_string(&format!(r"{key}\LocalServer32"));
                let handler32 = reg_default_string(&format!(r"{key}\InprocHandler32"));

                if name.is_none() && impl32.is_none() && impl_local32.is_none() && handler32.is_none() {
                    continue;
                }
                println!("[{label}]");
                if let Some(n) = name {
                    println!("  名称            : {n}");
                }
                if let Some(h) = handler32 {
                    println!("  InprocHandler32 : {h}   ← 无 InprocServer32，跨位数/代理宿主形态");
                }
                if let Some(d) = impl32 {
                    let c = clean_dll_path(&d);
                    println!("  InprocServer32  : {c}   exists={}", Path::new(&c).exists());
                }
                if let Some(d) = impl_local32 {
                    println!("  LocalServer32   : {}", clean_dll_path(&d));
                }
            }

            if let Some(appid) = reg_default_string(&format!(r"{k32}\AppID")) {
                println!("  AppID           : {appid}");
            }

            println!(
                "\n宿主提示：若该 CLSID 无 InprocServer32 且仅有 InprocHandler32/仅注册于 WOW6432Node,\n\
                 则它是 32 位组件，64 位宿主必须用 CLSCTX_LOCAL_SERVER 走 DCOM 代理（本程序已如此设置）。"
            );
        }
        None => {
            println!("\n[!] 三处注册点均未找到处理器 —— 该类型在本机没有可复用的 WPS/Office 预览器。");
        }
    }

    println!("════════════════════════════════════");
}

/// 把可选 GUID 格式化成人读字符串
fn fmt_guid(g: &Option<GUID>) -> String {
    match g {
        Some(g) => format!("{{{g:?}}}"),
        None => "<无>".into(),
    }
}

/// 去掉注册表 DLL 字符串里的引号与资源索引等尾巴
fn clean_dll_path(raw: &str) -> String {
    let t = raw.trim().trim_start_matches('"');
    let t = t.split(',').next().unwrap_or(t);
    let t = t.trim().trim_matches('"').trim();
    // 处理 "\"C:\path\file.dll\" /arg" 这类形式
    if let Some(idx) = t.find('"') {
        return t[..idx].to_string();
    }
    t.to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// 工具
// ─────────────────────────────────────────────────────────────────────────────

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// `GetModuleHandleW` 返回 `HMODULE`，而 `CreateWindowExW` 要 `HINSTANCE`。
/// windows 0.58 里这两个句柄都是 `*mut c_void` 包装，可直接取内部指针转换。
fn hinst_instance(m: windows::Win32::Foundation::HMODULE) -> windows::Win32::Foundation::HINSTANCE {
    windows::Win32::Foundation::HINSTANCE(m.0)
}

/// 注册表 API 成功的错误码（windows 0.58 未在 Registry 模块导出，故自定义）
const WIN32_ERROR_SUCCESS: u32 = 0;

fn absolute_path(p: &str) -> String {
    match std::fs::canonicalize(p) {
        Ok(c) => {
            let s = c.to_string_lossy().to_string();
            // canonicalize 在 Windows 上会加 \\?\ 前缀，处理器通常不接受
            s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
        }
        Err(_) => p.to_string(),
    }
}
