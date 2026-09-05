#![windows_subsystem = "windows"]
#![allow(unsafe_op_in_unsafe_fn)]

mod browsers;
mod config;
mod foreground;
mod installation;
mod registration;
mod routing;

use std::cell::RefCell;
use std::ffi::{OsStr, c_void};
use std::mem::{size_of, zeroed};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr::{null, null_mut};

use browsers::{BrowserInfo, installed_browsers};
use config::Config;
use routing::{PatternKind, RoutingRule};
use windows_sys::Win32::Foundation::{
    GlobalFree, HWND, LPARAM, LRESULT, POINT, SYSTEMTIME, WPARAM,
};
use windows_sys::Win32::Graphics::Gdi::{
    CLIP_DEFAULT_PRECIS, COLOR_WINDOW, COLOR_WINDOWTEXT, CreateFontW, DEFAULT_CHARSET,
    DEFAULT_GUI_FONT, DEFAULT_PITCH, DEFAULT_QUALITY, DeleteObject, FW_NORMAL, GetStockObject,
    GetSysColor, HDC, HFONT, NULL_BRUSH, OUT_DEFAULT_PRECIS, ScreenToClient, SetBkMode,
    SetTextColor, TRANSPARENT,
};
use windows_sys::Win32::System::DataExchange::{
    COPYDATASTRUCT, CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows_sys::Win32::System::SystemInformation::GetLocalTime;
use windows_sys::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, OFN_FILEMUSTEXIST, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows_sys::Win32::UI::Controls::{
    CBEIF_IMAGE, CBEIF_SELECTEDIMAGE, CBEIF_TEXT, CBEM_INSERTITEMW, CBEM_SETIMAGELIST,
    COMBOBOXEXITEMW, HIMAGELIST, ICC_LISTVIEW_CLASSES, ICC_TAB_CLASSES, ILC_COLOR32, ILC_MASK,
    INITCOMMONCONTROLSEX, ImageList_Create, ImageList_Destroy, ImageList_ReplaceIcon,
    InitCommonControlsEx, LVCF_TEXT, LVCF_WIDTH, LVCOLUMNW, LVHITTESTINFO, LVIF_IMAGE, LVIF_STATE,
    LVIF_TEXT, LVIS_FOCUSED, LVIS_SELECTED, LVITEMW, LVM_DELETEALLITEMS, LVM_ENSUREVISIBLE,
    LVM_GETNEXTITEM, LVM_HITTEST, LVM_INSERTCOLUMNW, LVM_INSERTITEMW, LVM_SETEXTENDEDLISTVIEWSTYLE,
    LVM_SETIMAGELIST, LVM_SETITEMSTATE, LVM_SETITEMTEXTW, LVN_BEGINDRAG, LVN_ITEMCHANGED,
    LVNI_SELECTED, LVS_EX_FULLROWSELECT, LVS_REPORT, LVS_SHOWSELALWAYS, LVS_SINGLESEL, LVSIL_SMALL,
    NM_CLICK, NM_RCLICK, NMHDR, NMLISTVIEW, TCIF_TEXT, TCITEMW, TCM_GETCURSEL, TCM_INSERTITEMW,
    TCM_SETCURSEL, TCN_SELCHANGE,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, ReleaseCapture, SetCapture, SetFocus,
};
use windows_sys::Win32::UI::Shell::{
    ExtractIconExW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
    Shell_NotifyIconW, ShellExecuteW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const CLASS_NAME: &str = "BrowserLauncherMainWindow";
const WINDOW_TITLE: &str = concat!("Browser Launcher (", env!("CARGO_PKG_VERSION"), ")");
const WM_TRAY: u32 = WM_APP + 1;
const WM_SHOW_SETTINGS: u32 = WM_APP + 2;

const ID_SETTINGS: usize = 1001;
const ID_COPY_LAST: usize = 1002;
const ID_EXIT: usize = 1003;
const ID_BROWSER_LIST: i32 = 1101;
const ID_BROWSER_ARGS: i32 = 1102;
const ID_BROWSE: usize = 1103;
const ID_SAVE: usize = 1104;
const ID_CANCEL: usize = 1105;
const ID_REGISTER_DEFAULT: usize = 1106;
const ID_SELECTION_LABEL: usize = 1107;
const ID_RULES_LIST: i32 = 1201;
const ID_RULE_KIND: i32 = 1202;
const ID_RULE_PATTERN: i32 = 1203;
const ID_RULE_BROWSER: i32 = 1204;
const ID_RULE_ARGUMENTS: i32 = 1205;
const ID_RULE_SAVE: usize = 1206;
const ID_RULE_DELETE: usize = 1207;
const ID_RULE_UP: usize = 1208;
const ID_RULE_DOWN: usize = 1209;
const ID_TABS: i32 = 1301;
const ID_HISTORY_LIST: i32 = 1302;
const ID_LANGUAGE: i32 = 1303;
const ID_HISTORY: usize = 1304;
const ID_HISTORY_STATUS: usize = 1305;
const ID_HISTORY_COPY_URL: usize = 1306;
const ID_HISTORY_COPY_ARGUMENTS: usize = 1307;
const ID_HISTORY_COPY_RECORD: usize = 1308;
const ID_HISTORY_TOAST_TIMER: usize = 1309;
const APP_ICON_ID: usize = 1;

thread_local! {
    static STATE: RefCell<Option<AppState>> = const { RefCell::new(None) };
}

struct AppState {
    config: Config,
    last_url: Option<String>,
    browser_list: HWND,
    args_edit: HWND,
    selection_label: HWND,
    rules_list: HWND,
    rule_kind: HWND,
    rule_pattern: HWND,
    rule_browser: HWND,
    rule_arguments: HWND,
    tabs: HWND,
    pages: Vec<Vec<HWND>>,
    history_list: HWND,
    history_status: HWND,
    language_combo: HWND,
    browsers: Vec<BrowserInfo>,
    routing_rules: Vec<RoutingRule>,
    history: Vec<HistoryEntry>,
    dragged_rule: Option<usize>,
    image_list: HIMAGELIST,
    editor_font: HFONT,
}

#[derive(Clone)]
struct HistoryEntry {
    time: String,
    url: String,
    browser: String,
    arguments: String,
}

#[derive(Clone, Copy)]
enum HistoryCopyTarget {
    Url,
    Arguments,
    Record,
}

fn main() {
    if command_line_has_option("--install-register") {
        if install_and_register().is_err() {
            std::process::exit(1);
        }
        return;
    }
    unsafe {
        let incoming_url = command_line_url();
        let existing = FindWindowW(wide(CLASS_NAME).as_ptr(), wide(WINDOW_TITLE).as_ptr());
        if !existing.is_null() {
            if let Some(url) = incoming_url {
                foreground::grant_foreground_activation();
                send_url(existing, &url);
            } else {
                PostMessageW(existing, WM_SHOW_SETTINGS, 0, 0);
            }
            return;
        }

        let instance = GetModuleHandleW(null());
        let controls = INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_LISTVIEW_CLASSES | ICC_TAB_CLASSES,
        };
        InitCommonControlsEx(&controls);
        let class_name = wide(CLASS_NAME);
        let cursor = LoadCursorW(null_mut(), IDC_ARROW);
        let icon = load_app_icon(instance);
        let window_class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            hIcon: icon,
            hCursor: cursor,
            hbrBackground: (COLOR_WINDOW + 1) as _,
            lpszClassName: class_name.as_ptr(),
            hIconSm: icon,
            ..zeroed()
        };
        if RegisterClassExW(&window_class) == 0 {
            show_error("Aplikaci se nepodařilo inicializovat.");
            return;
        }

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            wide(WINDOW_TITLE).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            780,
            570,
            null_mut(),
            null_mut(),
            instance,
            null(),
        );
        if hwnd.is_null() {
            show_error("Hlavní okno se nepodařilo vytvořit.");
            return;
        }

        add_tray_icon(hwnd);
        if let Some(url) = incoming_url {
            foreground::grant_foreground_activation();
            handle_url(hwnd, url);
        }

        let mut message: MSG = zeroed();
        while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_CREATE => {
            create_settings_controls(hwnd);
            0
        }
        WM_COMMAND => {
            match wparam & 0xffff {
                ID_SETTINGS => show_settings(hwnd),
                ID_HISTORY => show_settings_tab(hwnd, 2),
                ID_HISTORY_COPY_URL => copy_selected_history_value(hwnd, HistoryCopyTarget::Url),
                ID_HISTORY_COPY_ARGUMENTS => {
                    copy_selected_history_value(hwnd, HistoryCopyTarget::Arguments)
                }
                ID_HISTORY_COPY_RECORD => {
                    copy_selected_history_value(hwnd, HistoryCopyTarget::Record)
                }
                ID_COPY_LAST => copy_last_url(hwnd),
                ID_EXIT => {
                    DestroyWindow(hwnd);
                }
                ID_BROWSE => browse_for_browser(hwnd),
                ID_REGISTER_DEFAULT => register_as_default_browser(hwnd),
                ID_RULE_SAVE => save_or_update_rule(hwnd),
                ID_RULE_DELETE => delete_rule(),
                ID_RULE_UP => move_rule(-1),
                ID_RULE_DOWN => move_rule(1),
                ID_SAVE => save_settings(hwnd),
                ID_CANCEL => {
                    ShowWindow(hwnd, SW_HIDE);
                }
                _ => {}
            }
            0
        }
        WM_COPYDATA => {
            let data = &*(lparam as *const COPYDATASTRUCT);
            if data.dwData == 1 && data.cbData >= 2 && !data.lpData.is_null() {
                let units =
                    std::slice::from_raw_parts(data.lpData.cast::<u16>(), data.cbData as usize / 2);
                let end = units
                    .iter()
                    .position(|unit| *unit == 0)
                    .unwrap_or(units.len());
                handle_url(hwnd, String::from_utf16_lossy(&units[..end]));
            }
            1
        }
        WM_NOTIFY => {
            let notification = &*(lparam as *const NMHDR);
            if notification.idFrom == ID_TABS as usize && notification.code == TCN_SELCHANGE {
                show_selected_page();
            } else if notification.idFrom == ID_BROWSER_LIST as usize
                && notification.code == LVN_ITEMCHANGED
            {
                update_selection_label();
            } else if notification.idFrom == ID_RULES_LIST as usize
                && notification.code == LVN_ITEMCHANGED
            {
                load_selected_rule_into_editor();
            } else if notification.idFrom == ID_RULES_LIST as usize
                && notification.code == LVN_BEGINDRAG
            {
                begin_rule_drag();
            } else if notification.idFrom == ID_HISTORY_LIST as usize
                && notification.code == NM_CLICK
            {
                copy_history_cell(hwnd, &*(lparam as *const NMLISTVIEW));
            } else if notification.idFrom == ID_HISTORY_LIST as usize
                && notification.code == NM_RCLICK
            {
                show_history_context_menu(hwnd, &*(lparam as *const NMLISTVIEW));
            }
            0
        }
        WM_TIMER if wparam == ID_HISTORY_TOAST_TIMER => {
            KillTimer(hwnd, ID_HISTORY_TOAST_TIMER);
            STATE.with(|state| {
                if let Some(state) = state.borrow().as_ref() {
                    ShowWindow(state.history_status, SW_HIDE);
                }
            });
            0
        }
        WM_LBUTTONUP => {
            finish_rule_drag();
            0
        }
        WM_TRAY => {
            if lparam as u32 == WM_RBUTTONUP || lparam as u32 == WM_CONTEXTMENU {
                show_tray_menu(hwnd);
            } else if lparam as u32 == WM_LBUTTONDBLCLK {
                show_settings(hwnd);
            }
            0
        }
        WM_SHOW_SETTINGS => {
            show_settings(hwnd);
            0
        }
        WM_CTLCOLORSTATIC => {
            let hdc = wparam as HDC;
            SetBkMode(hdc, TRANSPARENT as i32);
            SetTextColor(hdc, GetSysColor(COLOR_WINDOWTEXT));
            GetStockObject(NULL_BRUSH) as LRESULT
        }
        WM_CLOSE => {
            ShowWindow(hwnd, SW_HIDE);
            0
        }
        WM_DESTROY => {
            remove_tray_icon(hwnd);
            STATE.with(|state| {
                if let Some(state) = state.borrow().as_ref()
                    && state.image_list != 0
                {
                    ImageList_Destroy(state.image_list);
                }
                if let Some(state) = state.borrow().as_ref()
                    && !state.editor_font.is_null()
                {
                    DeleteObject(state.editor_font as _);
                }
            });
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

unsafe fn create_settings_controls(hwnd: HWND) {
    let font = GetStockObject(DEFAULT_GUI_FONT);
    let editor_font_name = wide("Segoe UI");
    let editor_font = CreateFontW(
        -16,
        0,
        0,
        0,
        FW_NORMAL as i32,
        0,
        0,
        0,
        DEFAULT_CHARSET as u32,
        OUT_DEFAULT_PRECIS as u32,
        CLIP_DEFAULT_PRECIS as u32,
        DEFAULT_QUALITY as u32,
        DEFAULT_PITCH as u32,
        editor_font_name.as_ptr(),
    );
    let config = Config::load().unwrap_or_default();
    let english = config.language.eq_ignore_ascii_case("en");
    let tabs = create_control(
        hwnd,
        "SysTabControl32",
        "",
        10,
        10,
        744,
        470,
        WS_TABSTOP as i32,
        ID_TABS as usize,
    );
    for (index, label) in [
        tr(english, "Výchozí prohlížeč", "Default browser"),
        tr(english, "Pokročilé filtrování", "Advanced filtering"),
        tr(english, "Historie", "History"),
        tr(english, "Nastavení", "Settings"),
    ]
    .iter()
    .enumerate()
    {
        insert_tab(tabs, index, label);
    }
    let mut pages = vec![Vec::new(), Vec::new(), Vec::new(), Vec::new()];

    pages[0].push(create_control(
        hwnd,
        "STATIC",
        tr(
            english,
            "Použije se, pokud žádné pravidlo neodpovídá otevřené URL:",
            "Used when no rule matches the opened URL:",
        ),
        24,
        50,
        600,
        22,
        0,
        0,
    ));
    let browser_list = create_control(
        hwnd,
        "SysListView32",
        "",
        24,
        76,
        708,
        280,
        WS_BORDER as i32 | LVS_REPORT as i32 | LVS_SINGLESEL as i32 | LVS_SHOWSELALWAYS as i32,
        ID_BROWSER_LIST as usize,
    );
    pages[0].push(browser_list);
    let selection_label = create_control(
        hwnd,
        "STATIC",
        tr(
            english,
            "Kliknutím vyberte prohlížeč.",
            "Click to select a browser.",
        ),
        24,
        366,
        590,
        24,
        0,
        ID_SELECTION_LABEL,
    );
    pages[0].push(selection_label);
    pages[0].push(create_control(
        hwnd,
        "BUTTON",
        tr(english, "Procházet…", "Browse…"),
        632,
        363,
        100,
        26,
        BS_PUSHBUTTON,
        ID_BROWSE,
    ));
    pages[0].push(create_control(
        hwnd,
        "STATIC",
        tr(english, "Argumenty:", "Arguments:"),
        24,
        410,
        130,
        22,
        0,
        0,
    ));
    let args_edit = create_control(
        hwnd,
        "EDIT",
        "",
        154,
        407,
        578,
        24,
        WS_BORDER as i32 | ES_AUTOHSCROLL,
        ID_BROWSER_ARGS as usize,
    );
    pages[0].push(args_edit);
    pages[0].push(create_control(
        hwnd,
        "STATIC",
        tr(
            english,
            "Použijte {url}; pokud chybí, URL se přidá na konec.",
            "Use {url}; if omitted, the URL is appended.",
        ),
        154,
        437,
        578,
        22,
        0,
        0,
    ));

    pages[1].push(create_control(
        hwnd,
        "STATIC",
        tr(
            english,
            "První odpovídající pravidlo vyhrává. Tažením řádku změníte pořadí:",
            "The first matching rule wins. Drag rows to reorder:",
        ),
        24,
        50,
        680,
        22,
        0,
        0,
    ));
    let rules_list = create_control(
        hwnd,
        "SysListView32",
        "",
        24,
        76,
        708,
        240,
        WS_BORDER as i32 | LVS_REPORT as i32 | LVS_SINGLESEL as i32 | LVS_SHOWSELALWAYS as i32,
        ID_RULES_LIST as usize,
    );
    pages[1].push(rules_list);
    pages[1].push(create_control(
        hwnd,
        "STATIC",
        tr(english, "Typ:", "Type:"),
        24,
        329,
        120,
        20,
        0,
        0,
    ));
    let rule_kind = create_control(
        hwnd,
        "ComboBoxEx32",
        "",
        24,
        349,
        140,
        180,
        WS_TABSTOP as i32 | CBS_DROPDOWNLIST,
        ID_RULE_KIND as usize,
    );
    pages[1].push(rule_kind);
    pages[1].push(create_control(
        hwnd,
        "STATIC",
        tr(english, "Vzor URL:", "URL pattern:"),
        174,
        329,
        288,
        20,
        0,
        0,
    ));
    let rule_pattern = create_control(
        hwnd,
        "EDIT",
        "",
        174,
        349,
        288,
        30,
        WS_BORDER as i32 | ES_AUTOHSCROLL,
        ID_RULE_PATTERN as usize,
    );
    pages[1].push(rule_pattern);
    pages[1].push(create_control(
        hwnd,
        "STATIC",
        tr(english, "Prohlížeč:", "Browser:"),
        472,
        329,
        260,
        20,
        0,
        0,
    ));
    let rule_browser = create_control(
        hwnd,
        "ComboBoxEx32",
        "",
        472,
        349,
        260,
        220,
        WS_TABSTOP as i32 | CBS_DROPDOWNLIST,
        ID_RULE_BROWSER as usize,
    );
    pages[1].push(rule_browser);
    pages[1].push(create_control(
        hwnd,
        "STATIC",
        tr(english, "Argumenty:", "Arguments:"),
        24,
        389,
        130,
        20,
        0,
        0,
    ));
    let rule_arguments = create_control(
        hwnd,
        "EDIT",
        "",
        174,
        386,
        558,
        30,
        WS_BORDER as i32 | ES_AUTOHSCROLL,
        ID_RULE_ARGUMENTS as usize,
    );
    pages[1].push(rule_arguments);
    pages[1].push(create_control(
        hwnd,
        "BUTTON",
        tr(english, "Přidat / upravit", "Add / update"),
        24,
        430,
        145,
        30,
        BS_PUSHBUTTON,
        ID_RULE_SAVE,
    ));
    pages[1].push(create_control(
        hwnd,
        "BUTTON",
        tr(english, "Smazat", "Delete"),
        179,
        430,
        90,
        30,
        BS_PUSHBUTTON,
        ID_RULE_DELETE,
    ));
    pages[1].push(create_control(
        hwnd,
        "BUTTON",
        tr(english, "Nahoru", "Up"),
        542,
        430,
        90,
        30,
        BS_PUSHBUTTON,
        ID_RULE_UP,
    ));
    pages[1].push(create_control(
        hwnd,
        "BUTTON",
        tr(english, "Dolů", "Down"),
        642,
        430,
        90,
        30,
        BS_PUSHBUTTON,
        ID_RULE_DOWN,
    ));

    let history_list = create_control(
        hwnd,
        "SysListView32",
        "",
        24,
        50,
        708,
        370,
        WS_BORDER as i32 | LVS_REPORT as i32 | LVS_SINGLESEL as i32 | LVS_SHOWSELALWAYS as i32,
        ID_HISTORY_LIST as usize,
    );
    pages[2].push(history_list);
    let history_status = create_control(hwnd, "STATIC", "", 24, 430, 708, 22, 0, ID_HISTORY_STATUS);
    ShowWindow(history_status, SW_HIDE);
    pages[2].push(history_status);

    pages[3].push(create_control(
        hwnd,
        "STATIC",
        tr(english, "Integrace s Windows", "Windows integration"),
        24,
        58,
        300,
        22,
        0,
        0,
    ));
    pages[3].push(create_control(
        hwnd,
        "BUTTON",
        tr(
            english,
            "Nastavit jako výchozí prohlížeč…",
            "Set as default browser…",
        ),
        24,
        86,
        280,
        32,
        BS_PUSHBUTTON,
        ID_REGISTER_DEFAULT,
    ));
    pages[3].push(create_control(
        hwnd,
        "STATIC",
        tr(english, "Jazyk aplikace:", "Application language:"),
        24,
        155,
        150,
        22,
        0,
        0,
    ));
    let language_combo = create_control(
        hwnd,
        "COMBOBOX",
        "",
        174,
        151,
        130,
        120,
        WS_TABSTOP as i32 | CBS_DROPDOWNLIST,
        ID_LANGUAGE as usize,
    );
    pages[3].push(language_combo);
    pages[3].push(create_control(
        hwnd,
        "STATIC",
        tr(
            english,
            "Změna jazyka se projeví po příštím spuštění aplikace.",
            "The language change takes effect after the next app start.",
        ),
        24,
        190,
        600,
        22,
        0,
        0,
    ));

    create_control(
        hwnd,
        "BUTTON",
        tr(english, "Uložit", "Save"),
        552,
        492,
        95,
        32,
        BS_DEFPUSHBUTTON,
        ID_SAVE,
    );
    create_control(
        hwnd,
        "BUTTON",
        tr(english, "Zavřít", "Close"),
        657,
        492,
        95,
        32,
        BS_PUSHBUTTON,
        ID_CANCEL,
    );

    for child in [
        browser_list,
        args_edit,
        rules_list,
        rule_kind,
        rule_pattern,
        rule_browser,
        rule_arguments,
    ] {
        SendMessageW(child, WM_SETFONT, font as usize, 1);
    }
    EnumChildWindows(hwnd, Some(set_child_font), font as isize);
    if !editor_font.is_null() {
        for child in [rule_kind, rule_pattern, rule_browser, rule_arguments] {
            SendMessageW(child, WM_SETFONT, editor_font as usize, 1);
        }
    }

    let mut browsers = installed_browsers();
    if config.is_configured()
        && !browsers.iter().any(|browser| {
            browser
                .executable
                .eq_ignore_ascii_case(&config.browser_path)
        })
    {
        browsers.push(BrowserInfo {
            name: custom_browser_name(&config.browser_path),
            executable: config.browser_path.clone(),
            icon: format!("{},0", config.browser_path),
        });
    }
    for rule in &config.routing_rules {
        if !browsers
            .iter()
            .any(|browser| browser.executable.eq_ignore_ascii_case(&rule.browser_path))
        {
            browsers.push(BrowserInfo {
                name: custom_browser_name(&rule.browser_path),
                executable: rule.browser_path.clone(),
                icon: format!("{},0", rule.browser_path),
            });
        }
    }
    let image_list = ImageList_Create(
        24,
        24,
        ILC_COLOR32 | ILC_MASK,
        browsers.len().max(1) as i32,
        4,
    );
    SendMessageW(
        browser_list,
        LVM_SETIMAGELIST,
        LVSIL_SMALL as usize,
        image_list as isize,
    );
    SendMessageW(
        browser_list,
        LVM_SETEXTENDEDLISTVIEWSTYLE,
        0,
        LVS_EX_FULLROWSELECT as isize,
    );
    let mut heading = wide(tr(
        english,
        "Registrované prohlížeče",
        "Registered browsers",
    ));
    let mut column = LVCOLUMNW {
        mask: LVCF_TEXT | LVCF_WIDTH,
        cx: 560,
        pszText: heading.as_mut_ptr(),
        ..zeroed()
    };
    SendMessageW(
        browser_list,
        LVM_INSERTCOLUMNW,
        0,
        &mut column as *mut _ as isize,
    );
    for (index, browser) in browsers.iter().enumerate() {
        insert_browser_item(
            browser_list,
            image_list,
            index,
            browser,
            browser
                .executable
                .eq_ignore_ascii_case(&config.browser_path),
        );
    }
    set_text(args_edit, &config.browser_arguments);
    initialize_rules_controls(
        rules_list,
        rule_kind,
        rule_browser,
        image_list,
        &browsers,
        english,
    );
    SendMessageW(
        rules_list,
        LVM_SETIMAGELIST,
        LVSIL_SMALL as usize,
        image_list as isize,
    );
    SendMessageW(
        history_list,
        LVM_SETEXTENDEDLISTVIEWSTYLE,
        0,
        LVS_EX_FULLROWSELECT as isize,
    );
    insert_list_column(history_list, 0, tr(english, "Čas", "Time"), 72);
    insert_list_column(
        history_list,
        1,
        tr(
            english,
            "URL (kliknutím zkopírujete)",
            "URL (click to copy)",
        ),
        310,
    );
    insert_list_column(history_list, 2, tr(english, "Prohlížeč", "Browser"), 130);
    insert_list_column(history_list, 3, tr(english, "Argumenty", "Arguments"), 180);
    for label in ["Čeština", "English"] {
        SendMessageW(
            language_combo,
            CB_ADDSTRING,
            0,
            wide(label).as_ptr() as isize,
        );
    }
    SendMessageW(
        language_combo,
        CB_SETCURSEL,
        usize::from(config.language.eq_ignore_ascii_case("en")),
        0,
    );
    let routing_rules = config.routing_rules.clone();
    STATE.with(|state| {
        *state.borrow_mut() = Some(AppState {
            config,
            last_url: None,
            browser_list,
            args_edit,
            selection_label,
            rules_list,
            rule_kind,
            rule_pattern,
            rule_browser,
            rule_arguments,
            tabs,
            pages,
            history_list,
            history_status,
            language_combo,
            browsers,
            routing_rules,
            history: Vec::new(),
            dragged_rule: None,
            image_list,
            editor_font,
        });
    });
    update_selection_label();
    refresh_rules_list();
    show_selected_page();
}

unsafe fn insert_tab(tabs: HWND, index: usize, label: &str) {
    let mut text = wide(label);
    let mut item = TCITEMW {
        mask: TCIF_TEXT,
        pszText: text.as_mut_ptr(),
        ..zeroed()
    };
    SendMessageW(tabs, TCM_INSERTITEMW, index, &mut item as *mut _ as isize);
}

unsafe fn show_selected_page() {
    STATE.with(|state| {
        if let Some(state) = state.borrow().as_ref() {
            let selected = SendMessageW(state.tabs, TCM_GETCURSEL, 0, 0).max(0) as usize;
            for (page_index, controls) in state.pages.iter().enumerate() {
                for control in controls {
                    ShowWindow(
                        *control,
                        if page_index == selected {
                            SW_SHOW
                        } else {
                            SW_HIDE
                        },
                    );
                }
            }
        }
    });
}

unsafe extern "system" fn set_child_font(hwnd: HWND, font: LPARAM) -> i32 {
    SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
    1
}

#[allow(clippy::too_many_arguments)]
unsafe fn create_control(
    hwnd: HWND,
    class: &str,
    text: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    style: i32,
    id: usize,
) -> HWND {
    CreateWindowExW(
        0,
        wide(class).as_ptr(),
        wide(text).as_ptr(),
        WS_CHILD | WS_VISIBLE | style as u32,
        x,
        y,
        width,
        height,
        hwnd,
        id as _,
        GetModuleHandleW(null()),
        null(),
    )
}

unsafe fn show_settings(hwnd: HWND) {
    show_settings_tab(hwnd, 0);
}

unsafe fn show_settings_tab(hwnd: HWND, tab_index: usize) {
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            state.routing_rules = state.config.routing_rules.clone();
        }
    });
    clear_rule_editor();
    let values = STATE.with(|state| {
        if let Some(state) = state.borrow().as_ref() {
            let index = state.browsers.iter().position(|browser| {
                browser
                    .executable
                    .eq_ignore_ascii_case(&state.config.browser_path)
            });
            Some((
                state.args_edit,
                state.config.browser_arguments.clone(),
                state.browser_list,
                index,
            ))
        } else {
            None
        }
    });
    if let Some((args_edit, arguments, browser_list, index)) = values {
        set_text(args_edit, &arguments);
        if let Some(index) = index {
            select_browser(browser_list, index);
            SendMessageW(browser_list, LVM_ENSUREVISIBLE, index, 0);
        }
        update_selection_label();
        refresh_rules_list();
        SetFocus(browser_list);
    }
    STATE.with(|state| {
        if let Some(state) = state.borrow().as_ref() {
            SendMessageW(state.tabs, TCM_SETCURSEL, tab_index, 0);
        }
    });
    show_selected_page();
    ShowWindow(hwnd, SW_RESTORE);
    SetForegroundWindow(hwnd);
}

unsafe fn save_settings(hwnd: HWND) {
    let result = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut().unwrap();
        let Some(index) = selected_browser(state.browser_list) else {
            return Err("Vyberte prohlížeč ze seznamu.".to_owned());
        };
        let Some(browser) = state.browsers.get(index) else {
            return Err("Vybraný prohlížeč už není dostupný.".to_owned());
        };
        let language_index = SendMessageW(state.language_combo, CB_GETCURSEL, 0, 0);
        let config = Config {
            browser_path: browser.executable.clone(),
            browser_arguments: get_text(state.args_edit).trim().to_owned(),
            routing_rules: state.routing_rules.clone(),
            language: if language_index == 1 { "en" } else { "cs" }.to_owned(),
        };
        if !Path::new(&config.browser_path).is_file() {
            return Err("Vyberte existující spustitelný soubor prohlížeče.".to_owned());
        }
        config
            .save()
            .map_err(|error| format!("Nastavení se nepodařilo uložit: {error}"))?;
        state.config = config;
        Ok(())
    });
    match result {
        Ok(()) => {
            ShowWindow(hwnd, SW_HIDE);
        }
        Err(message) => message_box(hwnd, &message, MB_OK | MB_ICONWARNING),
    }
}

unsafe fn browse_for_browser(hwnd: HWND) {
    let mut file = [0u16; 32768];
    STATE.with(|state| {
        if let Some(state) = state.borrow().as_ref() {
            if let Some(index) = selected_browser(state.browser_list) {
                let current = &state.browsers[index].executable;
                let encoded: Vec<u16> = current.encode_utf16().collect();
                let count = encoded.len().min(file.len() - 1);
                file[..count].copy_from_slice(&encoded[..count]);
            }
        }
    });
    let filter = wide("Aplikace (*.exe)\0*.exe\0Všechny soubory\0*.*\0");
    let mut dialog: OPENFILENAMEW = zeroed();
    dialog.lStructSize = size_of::<OPENFILENAMEW>() as u32;
    dialog.hwndOwner = hwnd;
    dialog.lpstrFilter = filter.as_ptr();
    dialog.lpstrFile = file.as_mut_ptr();
    dialog.nMaxFile = file.len() as u32;
    dialog.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST;
    if GetOpenFileNameW(&mut dialog) != 0 {
        let selected = from_wide_null(&file);
        let selected_item = STATE.with(|state| {
            if let Some(state) = state.borrow_mut().as_mut() {
                let index = state
                    .browsers
                    .iter()
                    .position(|browser| browser.executable.eq_ignore_ascii_case(&selected));
                let index = index.unwrap_or_else(|| {
                    let index = state.browsers.len();
                    let browser = BrowserInfo {
                        name: custom_browser_name(&selected),
                        executable: selected.clone(),
                        icon: format!("{selected},0"),
                    };
                    insert_browser_item(
                        state.browser_list,
                        state.image_list,
                        index,
                        &browser,
                        false,
                    );
                    insert_combo_browser(state.rule_browser, index, &browser);
                    state.browsers.push(browser);
                    index
                });
                Some((state.browser_list, index))
            } else {
                None
            }
        });
        if let Some((list, index)) = selected_item {
            select_browser(list, index);
            update_selection_label();
            SetFocus(list);
        }
    }
}

unsafe fn initialize_rules_controls(
    rules_list: HWND,
    rule_kind: HWND,
    rule_browser: HWND,
    image_list: HIMAGELIST,
    browsers: &[BrowserInfo],
    english: bool,
) {
    SendMessageW(rule_kind, CBEM_SETIMAGELIST, 0, image_list);
    for (index, label) in [
        "Wildcard (*)",
        tr(english, "Regulární výraz", "Regular expression"),
        tr(english, "Výchozí", "Default"),
    ]
    .iter()
    .enumerate()
    {
        let mut text = wide(label);
        let mut item = COMBOBOXEXITEMW {
            mask: CBEIF_TEXT,
            iItem: index as isize,
            pszText: text.as_mut_ptr(),
            ..zeroed()
        };
        SendMessageW(rule_kind, CBEM_INSERTITEMW, 0, &mut item as *mut _ as isize);
    }
    SendMessageW(rule_kind, CB_SETCURSEL, 0, 0);
    SendMessageW(rule_browser, CBEM_SETIMAGELIST, 0, image_list);
    for (index, browser) in browsers.iter().enumerate() {
        insert_combo_browser(rule_browser, index, browser);
    }
    if !browsers.is_empty() {
        SendMessageW(rule_browser, CB_SETCURSEL, 0, 0);
    }

    SendMessageW(
        rules_list,
        LVM_SETEXTENDEDLISTVIEWSTYLE,
        0,
        LVS_EX_FULLROWSELECT as isize,
    );
    insert_list_column(rules_list, 0, tr(english, "Typ", "Type"), 90);
    insert_list_column(rules_list, 1, tr(english, "Vzor URL", "URL pattern"), 230);
    insert_list_column(rules_list, 2, tr(english, "Prohlížeč", "Browser"), 170);
    insert_list_column(rules_list, 3, tr(english, "Argumenty", "Arguments"), 195);
}

unsafe fn insert_combo_browser(combo: HWND, index: usize, browser: &BrowserInfo) {
    let mut text = wide(&browser.name);
    let mut item = COMBOBOXEXITEMW {
        mask: CBEIF_TEXT | CBEIF_IMAGE | CBEIF_SELECTEDIMAGE,
        iItem: index as isize,
        pszText: text.as_mut_ptr(),
        iImage: index as i32,
        iSelectedImage: index as i32,
        ..zeroed()
    };
    SendMessageW(combo, CBEM_INSERTITEMW, 0, &mut item as *mut _ as isize);
}

unsafe fn insert_list_column(list: HWND, index: usize, label: &str, width: i32) {
    let mut text = wide(label);
    let mut column = LVCOLUMNW {
        mask: LVCF_TEXT | LVCF_WIDTH,
        cx: width,
        pszText: text.as_mut_ptr(),
        ..zeroed()
    };
    SendMessageW(
        list,
        LVM_INSERTCOLUMNW,
        index,
        &mut column as *mut _ as isize,
    );
}

unsafe fn refresh_rules_list() {
    let values = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        let english = state.config.language.eq_ignore_ascii_case("en");
        let mut rows = state
            .routing_rules
            .iter()
            .map(|rule| {
                let kind = match rule.kind {
                    PatternKind::Wildcard => "Wildcard",
                    PatternKind::Regex => tr(english, "Regex", "Regex"),
                };
                let browser = browser_name_for_path(&state.browsers, &rule.browser_path);
                let icon = state
                    .browsers
                    .iter()
                    .position(|item| item.executable.eq_ignore_ascii_case(&rule.browser_path))
                    .unwrap_or(0);
                (
                    kind.to_owned(),
                    rule.pattern.clone(),
                    browser,
                    rule.browser_arguments.clone(),
                    icon,
                )
            })
            .collect::<Vec<_>>();
        let default_index = selected_browser(state.browser_list).unwrap_or(0);
        let default_path = state
            .browsers
            .get(default_index)
            .map_or("", |browser| browser.executable.as_str());
        let default_icon = default_index;
        rows.push((
            tr(english, "Výchozí", "Default").to_owned(),
            tr(
                english,
                "Vždy poslední fallback",
                "Always the last fallback",
            )
            .to_owned(),
            browser_name_for_path(&state.browsers, default_path),
            get_text(state.args_edit),
            default_icon,
        ));
        Some((state.rules_list, rows))
    });
    let Some((list, rows)) = values else {
        return;
    };
    SendMessageW(list, LVM_DELETEALLITEMS, 0, 0);
    for (index, (kind, pattern, browser, arguments, icon)) in rows.iter().enumerate() {
        insert_rule_row(list, index, kind, pattern, browser, arguments, *icon);
    }
}

unsafe fn insert_rule_row(
    list: HWND,
    index: usize,
    kind: &str,
    pattern: &str,
    browser: &str,
    arguments: &str,
    image: usize,
) {
    let mut kind_text = wide(kind);
    let mut item = LVITEMW {
        mask: LVIF_TEXT | LVIF_IMAGE,
        iItem: index as i32,
        pszText: kind_text.as_mut_ptr(),
        iImage: image as i32,
        ..zeroed()
    };
    SendMessageW(list, LVM_INSERTITEMW, 0, &mut item as *mut _ as isize);
    set_list_subitem(list, index, 1, pattern);
    set_list_subitem(list, index, 2, browser);
    set_list_subitem(list, index, 3, arguments);
}

unsafe fn set_list_subitem(list: HWND, row: usize, column: i32, value: &str) {
    let mut text = wide(value);
    let mut item = LVITEMW {
        iSubItem: column,
        pszText: text.as_mut_ptr(),
        ..zeroed()
    };
    SendMessageW(list, LVM_SETITEMTEXTW, row, &mut item as *mut _ as isize);
}

unsafe fn load_selected_rule_into_editor() {
    let values = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        let index = selected_browser(state.rules_list)?;
        if index == state.routing_rules.len() {
            let browser_index = selected_browser(state.browser_list).unwrap_or(0);
            return Some((
                state.rule_kind,
                state.rule_pattern,
                state.rule_browser,
                state.rule_arguments,
                None,
                browser_index,
                get_text(state.args_edit),
            ));
        }
        let rule = state.routing_rules.get(index)?.clone();
        let browser_index = state
            .browsers
            .iter()
            .position(|browser| browser.executable.eq_ignore_ascii_case(&rule.browser_path));
        Some((
            state.rule_kind,
            state.rule_pattern,
            state.rule_browser,
            state.rule_arguments,
            Some(rule.clone()),
            browser_index.unwrap_or(0),
            rule.browser_arguments,
        ))
    });
    let Some((kind, pattern, browser, arguments, rule, browser_index, argument_text)) = values
    else {
        return;
    };
    let is_default = rule.is_none();
    let kind_index = rule.as_ref().map_or(2, |rule| match rule.kind {
        PatternKind::Wildcard => 0,
        PatternKind::Regex => 1,
    });
    SendMessageW(kind, CB_SETCURSEL, kind_index, 0);
    set_text(
        pattern,
        rule.as_ref().map_or("", |rule| rule.pattern.as_str()),
    );
    set_text(arguments, &argument_text);
    SendMessageW(browser, CB_SETCURSEL, browser_index, 0);
    EnableWindow(kind, !is_default as i32);
    EnableWindow(pattern, !is_default as i32);
    STATE.with(|state| {
        if let Some(state) = state.borrow().as_ref() {
            EnableWindow(
                GetDlgItem(GetParent(state.rules_list), ID_RULE_DELETE as i32),
                !is_default as i32,
            );
            EnableWindow(
                GetDlgItem(GetParent(state.rules_list), ID_RULE_UP as i32),
                !is_default as i32,
            );
            EnableWindow(
                GetDlgItem(GetParent(state.rules_list), ID_RULE_DOWN as i32),
                !is_default as i32,
            );
        }
    });
}

unsafe fn clear_rule_editor() {
    let handles = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        let browser_index = state
            .browsers
            .iter()
            .position(|browser| {
                browser
                    .executable
                    .eq_ignore_ascii_case(&state.config.browser_path)
            })
            .unwrap_or(0);
        Some((
            state.rule_kind,
            state.rule_pattern,
            state.rule_browser,
            state.rule_arguments,
            browser_index,
        ))
    });
    if let Some((kind, pattern, browser, arguments, browser_index)) = handles {
        EnableWindow(kind, 1);
        EnableWindow(pattern, 1);
        SendMessageW(kind, CB_SETCURSEL, 0, 0);
        SendMessageW(browser, CB_SETCURSEL, browser_index, 0);
        set_text(pattern, "");
        set_text(arguments, "");
        let parent = GetParent(kind);
        EnableWindow(GetDlgItem(parent, ID_RULE_DELETE as i32), 1);
        EnableWindow(GetDlgItem(parent, ID_RULE_UP as i32), 1);
        EnableWindow(GetDlgItem(parent, ID_RULE_DOWN as i32), 1);
    }
}

unsafe fn save_or_update_rule(hwnd: HWND) {
    let handles = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref().unwrap();
        (
            state.rules_list,
            state.rule_kind,
            state.rule_pattern,
            state.rule_browser,
            state.rule_arguments,
            state.browsers.clone(),
            state.routing_rules.len(),
            state.browser_list,
            state.args_edit,
        )
    });
    let kind_index = SendMessageW(handles.1, CB_GETCURSEL, 0, 0);
    let browser_index = SendMessageW(handles.3, CB_GETCURSEL, 0, 0);
    if kind_index < 0 || browser_index < 0 {
        message_box(
            hwnd,
            "Vyberte typ vzoru a prohlížeč.",
            MB_OK | MB_ICONWARNING,
        );
        return;
    }
    let Some(browser) = handles.5.get(browser_index as usize) else {
        message_box(
            hwnd,
            "Vybraný prohlížeč už není dostupný.",
            MB_OK | MB_ICONWARNING,
        );
        return;
    };
    let selected = selected_browser(handles.0);
    if selected == Some(handles.6) {
        select_browser(handles.7, browser_index as usize);
        set_text(handles.8, get_text(handles.4).trim());
        update_selection_label();
        refresh_rules_list();
        select_browser(handles.0, handles.6);
        SetFocus(handles.0);
        return;
    }
    let rule = RoutingRule {
        kind: if kind_index == 0 {
            PatternKind::Wildcard
        } else {
            PatternKind::Regex
        },
        pattern: get_text(handles.2).trim().to_owned(),
        browser_path: browser.executable.clone(),
        browser_arguments: get_text(handles.4).trim().to_owned(),
    };
    if let Err(message) = rule.validate() {
        message_box(hwnd, &message, MB_OK | MB_ICONWARNING);
        return;
    }
    let index = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut().unwrap();
        if let Some(index) = selected {
            state.routing_rules[index] = rule;
            index
        } else {
            state.routing_rules.push(rule);
            state.routing_rules.len() - 1
        }
    });
    refresh_rules_list();
    select_browser(handles.0, index);
    SetFocus(handles.0);
}

unsafe fn delete_rule() {
    let values = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        let index = selected_browser(state.rules_list)?;
        (index < state.routing_rules.len()).then_some((state.rules_list, index))
    });
    let Some((list, index)) = values else {
        return;
    };
    let remaining = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut().unwrap();
        state.routing_rules.remove(index);
        state.routing_rules.len()
    });
    refresh_rules_list();
    if remaining > 0 {
        select_browser(list, index.min(remaining - 1));
    } else {
        clear_rule_editor();
    }
}

unsafe fn move_rule(direction: isize) {
    let values = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        Some((
            state.rules_list,
            selected_browser(state.rules_list)?,
            state.routing_rules.len(),
        ))
    });
    let Some((list, index, count)) = values else {
        return;
    };
    if index >= count {
        return;
    }
    let destination = index as isize + direction;
    if destination < 0 || destination >= count as isize {
        return;
    }
    let destination = destination as usize;
    STATE.with(|state| {
        state
            .borrow_mut()
            .as_mut()
            .unwrap()
            .routing_rules
            .swap(index, destination);
    });
    refresh_rules_list();
    select_browser(list, destination);
    SetFocus(list);
}

fn browser_name_for_path(browsers: &[BrowserInfo], path: &str) -> String {
    browsers
        .iter()
        .find(|browser| browser.executable.eq_ignore_ascii_case(path))
        .map_or_else(|| custom_browser_name(path), |browser| browser.name.clone())
}

unsafe fn insert_browser_item(
    list: HWND,
    image_list: HIMAGELIST,
    index: usize,
    browser: &BrowserInfo,
    selected: bool,
) {
    let extracted_icon = extract_browser_icon(&browser.icon);
    let icon = extracted_icon.unwrap_or_else(|| load_app_icon(GetModuleHandleW(null())));
    let image_index = ImageList_ReplaceIcon(image_list, -1, icon);
    if extracted_icon.is_some() {
        DestroyIcon(icon);
    }
    let mut text = wide(&browser.name);
    let selected_state = if selected {
        LVIS_SELECTED | LVIS_FOCUSED
    } else {
        0
    };
    let mut item = LVITEMW {
        mask: LVIF_TEXT | LVIF_IMAGE | LVIF_STATE,
        iItem: index as i32,
        pszText: text.as_mut_ptr(),
        iImage: image_index,
        state: selected_state,
        stateMask: LVIS_SELECTED | LVIS_FOCUSED,
        ..zeroed()
    };
    SendMessageW(list, LVM_INSERTITEMW, 0, &mut item as *mut _ as isize);
}

unsafe fn extract_browser_icon(specification: &str) -> Option<HICON> {
    let specification = specification.trim();
    if specification.is_empty() {
        return None;
    }
    let (path, index) = specification
        .rsplit_once(',')
        .and_then(|(path, index)| index.trim().parse::<i32>().ok().map(|index| (path, index)))
        .unwrap_or((specification, 0));
    let path = path.trim().trim_matches('"');
    let mut icon = null_mut();
    if ExtractIconExW(wide(path).as_ptr(), index, null_mut(), &mut icon, 1) == 0 || icon.is_null() {
        None
    } else {
        Some(icon)
    }
}

unsafe fn selected_browser(list: HWND) -> Option<usize> {
    let index = SendMessageW(list, LVM_GETNEXTITEM, usize::MAX, LVNI_SELECTED as isize);
    (index >= 0).then_some(index as usize)
}

unsafe fn select_browser(list: HWND, index: usize) {
    let mut clear = LVITEMW {
        state: 0,
        stateMask: LVIS_SELECTED | LVIS_FOCUSED,
        ..zeroed()
    };
    SendMessageW(
        list,
        LVM_SETITEMSTATE,
        usize::MAX,
        &mut clear as *mut _ as isize,
    );
    let mut item = LVITEMW {
        state: LVIS_SELECTED | LVIS_FOCUSED,
        stateMask: LVIS_SELECTED | LVIS_FOCUSED,
        ..zeroed()
    };
    SendMessageW(list, LVM_SETITEMSTATE, index, &mut item as *mut _ as isize);
}

unsafe fn update_selection_label() {
    let values = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        let index = selected_browser(state.browser_list);
        let english = state.config.language.eq_ignore_ascii_case("en");
        let text = index
            .and_then(|index| state.browsers.get(index))
            .map_or_else(
                || {
                    tr(
                        english,
                        "Kliknutím vyberte prohlížeč.",
                        "Click to select a browser.",
                    )
                    .to_owned()
                },
                |browser| {
                    if english {
                        format!("✓ Selected: {} — confirm with Save", browser.name)
                    } else {
                        format!("✓ Vybráno: {} — potvrďte tlačítkem Uložit", browser.name)
                    }
                },
            );
        Some((state.selection_label, text))
    });
    if let Some((label, text)) = values {
        set_text(label, &text);
    }
}

fn custom_browser_name(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|name| name.to_str())
        .map_or_else(
            || "Vlastní prohlížeč".to_owned(),
            |name| format!("{name} (vlastní)"),
        )
}

unsafe fn register_as_default_browser(hwnd: HWND) {
    let installation = install_and_register();
    if let Err(error) = installation {
        message_box(
            hwnd,
            &format!("Registrace ve Windows se nezdařila: {error}"),
            MB_OK | MB_ICONERROR,
        );
        return;
    }
    let settings_uri = "ms-settings:defaultapps?registeredAppUser=Browser%20Launcher";
    let result = ShellExecuteW(
        hwnd,
        wide("open").as_ptr(),
        wide(settings_uri).as_ptr(),
        null(),
        null(),
        SW_SHOWNORMAL,
    );
    if result as isize <= 32 {
        ShellExecuteW(
            hwnd,
            wide("open").as_ptr(),
            wide("ms-settings:defaultapps").as_ptr(),
            null(),
            null(),
            SW_SHOWNORMAL,
        );
    }
}

fn install_and_register() -> std::io::Result<()> {
    let executable = installation::install_current_user()?;
    registration::register_current_user(&executable)
}

unsafe fn handle_url(hwnd: HWND, url: String) {
    let config = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut().unwrap();
        state.last_url = Some(url.clone());
        state.config.clone()
    });
    if !config.is_configured() {
        return;
    }
    let target = routing::resolve(
        &config.routing_rules,
        &config.browser_path,
        &config.browser_arguments,
        &url,
    );
    if !Path::new(target.path).is_file() {
        return;
    }
    let browser_path = target.path.to_owned();
    let parameters = build_browser_arguments(target.arguments, &url);
    let result = ShellExecuteW(
        hwnd,
        wide("open").as_ptr(),
        wide(&browser_path).as_ptr(),
        wide(&parameters).as_ptr(),
        null(),
        SW_SHOWNORMAL,
    );
    if result as isize <= 32 {
        message_box(
            hwnd,
            "Prohlížeč se nepodařilo spustit.",
            MB_OK | MB_ICONERROR,
        );
    } else {
        record_history(&url, &browser_path, &parameters);
        foreground::bring_browser_to_front(browser_path);
    }
}

unsafe fn record_history(url: &str, browser_path: &str, arguments: &str) {
    let mut time: SYSTEMTIME = zeroed();
    GetLocalTime(&mut time);
    let timestamp = format!("{:02}:{:02}:{:02}", time.wHour, time.wMinute, time.wSecond);
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            let browser = browser_name_for_path(&state.browsers, browser_path);
            state.history.insert(
                0,
                HistoryEntry {
                    time: timestamp,
                    url: url.to_owned(),
                    browser,
                    arguments: arguments.to_owned(),
                },
            );
            state.history.truncate(500);
        }
    });
    refresh_history_list();
}

unsafe fn refresh_history_list() {
    let values = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        Some((state.history_list, state.history.clone()))
    });
    let Some((list, history)) = values else {
        return;
    };
    SendMessageW(list, LVM_DELETEALLITEMS, 0, 0);
    for (index, entry) in history.iter().enumerate() {
        let mut time = wide(&entry.time);
        let mut item = LVITEMW {
            mask: LVIF_TEXT,
            iItem: index as i32,
            pszText: time.as_mut_ptr(),
            ..zeroed()
        };
        SendMessageW(list, LVM_INSERTITEMW, 0, &mut item as *mut _ as isize);
        set_list_subitem(list, index, 1, &entry.url);
        set_list_subitem(list, index, 2, &entry.browser);
        set_list_subitem(list, index, 3, &entry.arguments);
    }
}

unsafe fn copy_history_cell(hwnd: HWND, notification: &NMLISTVIEW) {
    if notification.iItem < 0 {
        return;
    }
    let index = notification.iItem as usize;
    STATE.with(|state| {
        if let Some(state) = state.borrow().as_ref() {
            select_browser(state.history_list, index);
        }
    });
    match notification.iSubItem {
        1 => copy_history_value_at(hwnd, index, HistoryCopyTarget::Url),
        3 => copy_history_value_at(hwnd, index, HistoryCopyTarget::Arguments),
        _ => {}
    }
}

unsafe fn copy_selected_history_value(hwnd: HWND, target: HistoryCopyTarget) {
    let index = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        selected_browser(state.history_list)
    });
    if let Some(index) = index {
        copy_history_value_at(hwnd, index, target);
    }
}

unsafe fn copy_history_value_at(hwnd: HWND, index: usize, target: HistoryCopyTarget) {
    let value = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        let entry = state.history.get(index)?;
        let value = match target {
            HistoryCopyTarget::Url => entry.url.clone(),
            HistoryCopyTarget::Arguments => entry.arguments.clone(),
            HistoryCopyTarget::Record => format!(
                "{}\t{}\t{}\t{}",
                entry.time, entry.url, entry.browser, entry.arguments
            ),
        };
        Some((
            value,
            state.history_status,
            state.config.language.eq_ignore_ascii_case("en"),
        ))
    });
    let Some((value, status, english)) = value else {
        return;
    };
    if let Err(message) = set_clipboard_text(hwnd, &value) {
        message_box(hwnd, &message, MB_OK | MB_ICONERROR);
        return;
    }
    let toast = match target {
        HistoryCopyTarget::Url => tr(
            english,
            "URL zkopírována do schránky.",
            "URL copied to clipboard.",
        ),
        HistoryCopyTarget::Arguments => tr(
            english,
            "Argumenty zkopírovány do schránky.",
            "Arguments copied to clipboard.",
        ),
        HistoryCopyTarget::Record => tr(
            english,
            "Celý záznam zkopírován do schránky.",
            "Full record copied to clipboard.",
        ),
    };
    set_text(status, toast);
    ShowWindow(status, SW_SHOW);
    SetTimer(hwnd, ID_HISTORY_TOAST_TIMER, 2500, None);
}

unsafe fn show_history_context_menu(hwnd: HWND, notification: &NMLISTVIEW) {
    if notification.iItem < 0 {
        return;
    }
    let index = notification.iItem as usize;
    let english = STATE.with(|state| {
        let state = state.borrow();
        let state = state.as_ref()?;
        select_browser(state.history_list, index);
        Some(state.config.language.eq_ignore_ascii_case("en"))
    });
    let Some(english) = english else {
        return;
    };
    let menu = CreatePopupMenu();
    AppendMenuW(
        menu,
        MF_STRING,
        ID_HISTORY_COPY_URL,
        wide(tr(english, "Kopírovat URL", "Copy URL")).as_ptr(),
    );
    AppendMenuW(
        menu,
        MF_STRING,
        ID_HISTORY_COPY_ARGUMENTS,
        wide(tr(english, "Kopírovat argumenty", "Copy arguments")).as_ptr(),
    );
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    AppendMenuW(
        menu,
        MF_STRING,
        ID_HISTORY_COPY_RECORD,
        wide(tr(english, "Kopírovat celý záznam", "Copy full record")).as_ptr(),
    );
    let mut point = POINT::default();
    GetCursorPos(&mut point);
    SetForegroundWindow(hwnd);
    TrackPopupMenu(menu, TPM_RIGHTBUTTON, point.x, point.y, 0, hwnd, null());
    DestroyMenu(menu);
}

unsafe fn begin_rule_drag() {
    STATE.with(|state| {
        if let Some(state) = state.borrow_mut().as_mut() {
            let selected = selected_browser(state.rules_list);
            state.dragged_rule = selected.filter(|index| *index < state.routing_rules.len());
            if state.dragged_rule.is_some() {
                SetCapture(GetParent(state.rules_list));
            }
        }
    });
}

unsafe fn finish_rule_drag() {
    let values = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut()?;
        Some((
            state.rules_list,
            state.dragged_rule.take()?,
            state.routing_rules.len(),
        ))
    });
    let Some((list, source, count)) = values else {
        return;
    };
    ReleaseCapture();
    let mut point = POINT::default();
    GetCursorPos(&mut point);
    ScreenToClient(list, &mut point);
    let mut hit: LVHITTESTINFO = zeroed();
    hit.pt = point;
    let target = SendMessageW(list, LVM_HITTEST, 0, &mut hit as *mut _ as isize);
    if target < 0 {
        return;
    }
    let target = (target as usize).min(count.saturating_sub(1));
    if source == target {
        return;
    }
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut().unwrap();
        let rule = state.routing_rules.remove(source);
        state.routing_rules.insert(target, rule);
    });
    refresh_rules_list();
    select_browser(list, target);
}

fn build_browser_arguments(template: &str, url: &str) -> String {
    let quoted_url = quote_argument(url);
    if template.contains("{url}") {
        template.replace("{url}", &quoted_url)
    } else if template.trim().is_empty() {
        quoted_url
    } else {
        format!("{} {}", template.trim(), quoted_url)
    }
}

fn quote_argument(value: &str) -> String {
    let mut result = String::with_capacity(value.len() + 2);
    result.push('"');
    let mut backslashes = 0;
    for character in value.chars() {
        match character {
            '\\' => backslashes += 1,
            '"' => {
                result.push_str(&"\\".repeat(backslashes * 2 + 1));
                result.push('"');
                backslashes = 0;
            }
            _ => {
                result.push_str(&"\\".repeat(backslashes));
                backslashes = 0;
                result.push(character);
            }
        }
    }
    result.push_str(&"\\".repeat(backslashes * 2));
    result.push('"');
    result
}

unsafe fn copy_last_url(hwnd: HWND) {
    let url = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .and_then(|state| state.last_url.clone())
    });
    match url {
        Some(url) => {
            if let Err(message) = set_clipboard_text(hwnd, &url) {
                message_box(hwnd, &message, MB_OK | MB_ICONERROR);
            }
        }
        None => message_box(
            hwnd,
            "Zatím nebyla otevřena žádná URL.",
            MB_OK | MB_ICONINFORMATION,
        ),
    }
}

unsafe fn set_clipboard_text(hwnd: HWND, text: &str) -> Result<(), String> {
    if OpenClipboard(hwnd) == 0 {
        return Err("Schránku se nepodařilo otevřít.".to_owned());
    }
    EmptyClipboard();
    let bytes = wide(text);
    let memory = GlobalAlloc(GMEM_MOVEABLE, bytes.len() * 2);
    if memory.is_null() {
        CloseClipboard();
        return Err("Pro schránku se nepodařilo alokovat paměť.".to_owned());
    }
    let target = GlobalLock(memory).cast::<u16>();
    if target.is_null() {
        GlobalFree(memory);
        CloseClipboard();
        return Err("Pro schránku se nepodařilo zamknout paměť.".to_owned());
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), target, bytes.len());
    GlobalUnlock(memory);
    if SetClipboardData(13, memory).is_null() {
        GlobalFree(memory);
        CloseClipboard();
        return Err("Text se nepodařilo vložit do schránky.".to_owned());
    }
    CloseClipboard();
    Ok(())
}

unsafe fn show_tray_menu(hwnd: HWND) {
    let menu = CreatePopupMenu();
    let (has_last, english) = STATE.with(|state| {
        state.borrow().as_ref().map_or((false, false), |state| {
            (
                state.last_url.is_some(),
                state.config.language.eq_ignore_ascii_case("en"),
            )
        })
    });
    AppendMenuW(
        menu,
        MF_STRING,
        ID_SETTINGS,
        wide(tr(english, "Nastavení", "Settings")).as_ptr(),
    );
    AppendMenuW(
        menu,
        MF_STRING,
        ID_HISTORY,
        wide(tr(english, "Historie", "History")).as_ptr(),
    );
    AppendMenuW(
        menu,
        MF_STRING | if has_last { 0 } else { MF_GRAYED },
        ID_COPY_LAST,
        wide(tr(english, "Kopírovat poslední URL", "Copy last URL")).as_ptr(),
    );
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    AppendMenuW(
        menu,
        MF_STRING,
        ID_EXIT,
        wide(tr(english, "Ukončit", "Exit")).as_ptr(),
    );
    let mut point = POINT::default();
    GetCursorPos(&mut point);
    SetForegroundWindow(hwnd);
    TrackPopupMenu(menu, TPM_RIGHTBUTTON, point.x, point.y, 0, hwnd, null());
    DestroyMenu(menu);
}

unsafe fn add_tray_icon(hwnd: HWND) {
    let mut data: NOTIFYICONDATAW = zeroed();
    data.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = 1;
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    data.uCallbackMessage = WM_TRAY;
    data.hIcon = load_app_icon(GetModuleHandleW(null()));
    write_fixed_wide(&mut data.szTip, WINDOW_TITLE);
    Shell_NotifyIconW(NIM_ADD, &data);
}

unsafe fn load_app_icon(instance: windows_sys::Win32::Foundation::HINSTANCE) -> HICON {
    let icon = LoadIconW(instance, APP_ICON_ID as *const u16);
    if icon.is_null() {
        LoadIconW(null_mut(), IDI_APPLICATION)
    } else {
        icon
    }
}

unsafe fn remove_tray_icon(hwnd: HWND) {
    let mut data: NOTIFYICONDATAW = zeroed();
    data.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = 1;
    Shell_NotifyIconW(NIM_DELETE, &data);
}

unsafe fn send_url(hwnd: HWND, url: &str) {
    let encoded = wide(url);
    let data = COPYDATASTRUCT {
        dwData: 1,
        cbData: (encoded.len() * 2) as u32,
        lpData: encoded.as_ptr().cast::<c_void>() as *mut c_void,
    };
    SendMessageW(hwnd, WM_COPYDATA, 0, &data as *const _ as isize);
}

fn command_line_url() -> Option<String> {
    std::env::args_os().skip(1).find_map(|argument| {
        let value = argument.to_string_lossy().trim().to_owned();
        if value.is_empty() || value.starts_with('-') {
            None
        } else {
            Some(value)
        }
    })
}

fn command_line_has_option(option: &str) -> bool {
    std::env::args_os()
        .skip(1)
        .any(|argument| argument.eq_ignore_ascii_case(option))
}

unsafe fn get_text(hwnd: HWND) -> String {
    let length = GetWindowTextLengthW(hwnd);
    let mut buffer = vec![0u16; length as usize + 1];
    GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
    String::from_utf16_lossy(&buffer[..length as usize])
}

unsafe fn set_text(hwnd: HWND, text: &str) {
    SetWindowTextW(hwnd, wide(text).as_ptr());
}

unsafe fn message_box(hwnd: HWND, message: &str, flags: MESSAGEBOX_STYLE) {
    MessageBoxW(
        hwnd,
        wide(message).as_ptr(),
        wide(WINDOW_TITLE).as_ptr(),
        flags,
    );
}

unsafe fn show_error(message: &str) {
    message_box(null_mut(), message, MB_OK | MB_ICONERROR);
}

fn wide(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(Some(0)).collect()
}

fn tr<'a>(english: bool, czech: &'a str, english_text: &'a str) -> &'a str {
    if english { english_text } else { czech }
}

fn from_wide_null(value: &[u16]) -> String {
    let end = value
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(value.len());
    String::from_utf16_lossy(&value[..end])
}

fn write_fixed_wide<const N: usize>(target: &mut [u16; N], value: &str) {
    let encoded: Vec<u16> = value.encode_utf16().collect();
    let count = encoded.len().min(N - 1);
    target[..count].copy_from_slice(&encoded[..count]);
}

#[cfg(test)]
mod tests {
    use super::{build_browser_arguments, quote_argument};

    #[test]
    fn appends_url_when_template_has_no_placeholder() {
        assert_eq!(
            build_browser_arguments("--new-window", "https://example.com/a?b=1"),
            "--new-window \"https://example.com/a?b=1\""
        );
    }

    #[test]
    fn replaces_url_placeholder() {
        assert_eq!(
            build_browser_arguments("--app={url}", "https://example.com"),
            "--app=\"https://example.com\""
        );
    }

    #[test]
    fn quotes_embedded_quotes_and_trailing_backslashes() {
        assert_eq!(quote_argument("a\"b\\"), "\"a\\\"b\\\\\"");
    }
}
