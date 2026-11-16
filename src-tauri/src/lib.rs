// Clipsa - secure cross-platform clipboard manager.

pub mod clipboard;
pub mod crypto;
pub mod error;
pub mod lock;
pub mod platform;
pub mod settings;
pub mod storage;
#[cfg(feature = "self-update")]
mod updater;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use tauri::menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, WindowEvent, Wry};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tracing::{error, info, warn};

use clipboard::{ClipboardService, ContentType, HistoryPage, HistoryQuery, ItemView, StatsView};
use crypto::{CryptoManager, KeyStore};
use error::{ClipsaError, ClipsaResult};
use lock::{AppLock, LockState};
use platform::{Permissions, Platform};
use settings::AppSettings;
use storage::ClipboardStorage;

const MAIN_WINDOW: &str = "main";
const PICKER_WINDOW: &str = "picker";
const EVENT_CLIPBOARD_CHANGED: &str = "clipboard://changed";
const EVENT_SETTINGS_CHANGED: &str = "settings://changed";
const EVENT_PICKER_SHOWN: &str = "picker://shown";
const EVENT_LOCK_CHANGED: &str = "lock://changed";
const EVENT_MAIN_NAVIGATE: &str = "main://navigate";
/// Passed by the launch-at-login entry so Clipsa starts in the tray.
#[cfg(not(feature = "app-store"))]
const LAUNCHED_AT_LOGIN_ARG: &str = "--launched-at-login";

struct AppState {
    service: Arc<ClipboardService>,
    storage: Arc<ClipboardStorage>,
    platform: Arc<Platform>,
    settings: Arc<RwLock<AppSettings>>,
    /// Serialises settings updates (hotkey/autostart side effects + persist).
    settings_update: Mutex<()>,
    tray: TrayItems,
    lock: Arc<AppLock>,
}

impl AppState {
    fn settings(&self) -> AppSettings {
        self.settings
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    fn lock_state(&self) -> LockState {
        LockState {
            supported: self.platform.auth_supported(),
            locked: self.lock.is_locked(),
        }
    }
}

fn emit_lock_state(app: &AppHandle, state: &AppState) -> LockState {
    let lock_state = state.lock_state();
    let _ = app.emit(EVENT_LOCK_CHANGED, lock_state);
    lock_state
}

/// Release builds keep data under `<data dir>/clipsa` with the key in the OS
/// keychain. Debug builds use `<data dir>/clipsa-dev` and a key file, so
/// development never touches real history or triggers keychain prompts.
fn data_dir() -> ClipsaResult<PathBuf> {
    let base = dirs::data_dir()
        .ok_or_else(|| ClipsaError::Storage("Could not determine data directory".into()))?;
    Ok(base.join(if cfg!(debug_assertions) {
        "clipsa-dev"
    } else {
        "clipsa"
    }))
}

fn key_store(data_dir: &std::path::Path) -> ClipsaResult<Box<dyn KeyStore>> {
    if cfg!(debug_assertions) {
        Ok(Box::new(crypto::FileKeyStore::new(
            data_dir.join("dev.key"),
        )))
    } else {
        Ok(Box::new(crypto::KeychainStore::new()?))
    }
}

pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init());
    // Direct-download builds update themselves; App Store builds are updated
    // by the App Store and must not contain an updater.
    #[cfg(feature = "self-update")]
    let builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    // Launch at login: see `set_autostart` (LaunchAgent/registry/XDG entry for
    // direct builds, SMAppService for the App Store edition).
    let app = builder
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_picker(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            start(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| match (window.label(), event) {
            (MAIN_WINDOW, WindowEvent::CloseRequested { api, .. }) => {
                // Keep running in the tray; quit from the tray menu.
                api.prevent_close();
                let _ = window.hide();
            }
            #[cfg(feature = "self-update")]
            (updater::WINDOW, WindowEvent::CloseRequested { api, .. }) => {
                // Hide only: the window keeps the periodic update check running.
                api.prevent_close();
                let _ = window.hide();
            }
            (PICKER_WINDOW, WindowEvent::Focused(false)) => {
                // The OS authentication prompt takes focus; keep the picker up.
                let authenticating = window
                    .try_state::<AppState>()
                    .is_some_and(|state| state.lock.is_authenticating());
                if !authenticating {
                    let _ = window.hide();
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            get_history,
            get_item_content,
            copy_item,
            paste_item,
            delete_item,
            set_pinned,
            create_snippet,
            update_item,
            clear_history,
            get_statistics,
            get_settings,
            update_settings,
            check_permissions,
            request_permissions,
            hide_picker,
            open_main_window,
            get_version,
            get_build_info,
            get_lock_state,
            unlock,
            lock_now,
            #[cfg(feature = "self-update")]
            updater::show_updater,
            #[cfg(feature = "self-update")]
            updater::update_updater_settings,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build Tauri application");

    app.run(|app, event| {
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = event {
            show_main_window(app);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (app, event);
    });
}

/// Initialise the app. On failure, explain and offer to retry (keychain
/// errors, e.g. a denied or dismissed access prompt) or quit.
fn start(app: &AppHandle) {
    let Err(e) = initialize(app) else {
        return;
    };
    error!("Startup failed: {e}");
    for window in app.webview_windows().values() {
        let _ = window.hide();
    }
    // Keychain access fails before anything is set up, so retrying is safe.
    let retryable = matches!(e, ClipsaError::KeyManagement(_));
    let buttons = if retryable {
        MessageDialogButtons::OkCancelCustom("Try Again".into(), "Quit".into())
    } else {
        MessageDialogButtons::OkCustom("Quit".into())
    };
    let handle = app.clone();
    app.dialog()
        .message(startup_error_message(&e))
        .title("Clipsa could not start")
        .kind(MessageDialogKind::Error)
        .buttons(buttons)
        .show(move |try_again| {
            if retryable && try_again {
                let main = handle.clone();
                let _ = handle.run_on_main_thread(move || start(&main));
            } else {
                handle.exit(1);
            }
        });
}

fn startup_error_message(e: &ClipsaError) -> String {
    match e {
        ClipsaError::KeyManagement(detail) => format!(
            "Clipsa could not access its encryption key in the system keychain, \
             so it can't open your history. Nothing has been changed or deleted.\n\n\
             Click Try Again, then choose \"Always Allow\" when macOS asks for \
             keychain access.\n\nDetails: {detail}"
        ),
        other => format!("{other}"),
    }
}

fn initialize(app: &AppHandle) -> ClipsaResult<()> {
    let data_dir = data_dir()?;
    let crypto = Arc::new(CryptoManager::load_or_create(
        key_store(&data_dir)?.as_ref(),
    )?);
    let storage = Arc::new(ClipboardStorage::open(&ClipboardStorage::default_path(
        &data_dir,
    ))?);
    let settings = Arc::new(RwLock::new(storage.load_settings()?));
    let platform = Arc::new(Platform::new());

    let emitter = app.clone();
    let service = Arc::new(ClipboardService::new(
        Arc::clone(&platform),
        crypto,
        Arc::clone(&storage),
        Arc::clone(&settings),
        Box::new(move || {
            let _ = emitter.emit(EVENT_CLIPBOARD_CHANGED, ());
        }),
    ));
    service.upgrade_legacy_items()?;

    let initial = settings.read().unwrap_or_else(|p| p.into_inner()).clone();
    let tray = build_tray(app, &initial)?;
    #[cfg(target_os = "macos")]
    if let Err(e) = install_app_menu(app) {
        warn!("Could not add Settings to the app menu: {e}");
    }

    let lock = Arc::new(AppLock::new(initial.app_lock && platform.auth_supported()));
    app.manage(AppState {
        service: Arc::clone(&service),
        storage,
        platform,
        settings,
        settings_update: Mutex::new(()),
        tray,
        lock,
    });

    if let Err(e) = register_hotkey(app, &initial.global_hotkey) {
        warn!("Global hotkey `{}` unavailable: {e}", initial.global_hotkey);
    }
    #[cfg(not(feature = "app-store"))]
    match autostart_entry(app) {
        Ok(entry) => {
            app.manage(entry);
        }
        Err(e) => warn!("Launch at login unavailable: {e}"),
    }
    if let Err(e) = set_autostart(app, initial.launch_at_login) {
        warn!("Could not sync launch-at-login: {e}");
    }

    service.spawn_background();
    spawn_lock_timer(app.clone());
    #[cfg(feature = "self-update")]
    updater::create_window(app);

    // Started at login: stay in the tray. Otherwise (or before the first-run
    // screen was completed) show the main window.
    if !(launched_at_login(&initial) && initial.onboarding_completed) {
        show_main_window(app);
    }
    info!("Clipsa initialised (data in {})", data_dir.display());
    Ok(())
}

/// Whether this process was started by the launch-at-login entry.
#[cfg(not(feature = "app-store"))]
fn launched_at_login(_settings: &AppSettings) -> bool {
    std::env::args().any(|arg| arg == LAUNCHED_AT_LOGIN_ARG)
}

/// SMAppService login items cannot pass arguments, so the App Store edition
/// treats every start as a login start while launch at login is on and
/// registered: Clipsa then always starts in the tray (opening it again shows
/// the window).
#[cfg(feature = "app-store")]
fn launched_at_login(settings: &AppSettings) -> bool {
    settings.launch_at_login && platform::login_item::is_enabled()
}

/// Locks Clipsa once it has been in the background for `lock_after_minutes`
/// (no window focused) or, with 0, as soon as every window is hidden.
fn spawn_lock_timer(app: AppHandle) {
    std::thread::Builder::new()
        .name("clipsa-lock".into())
        .spawn(move || loop {
            std::thread::sleep(Duration::from_secs(1));
            let state = app.state::<AppState>();
            let settings = state.settings();
            let windows = app.webview_windows();
            let active = if settings.lock_after_minutes == 0 {
                windows.values().any(|w| w.is_visible().unwrap_or(false))
            } else {
                windows.values().any(|w| w.is_focused().unwrap_or(false))
            };
            let lock_after = Duration::from_secs(u64::from(settings.lock_after_minutes) * 60);
            if state
                .lock
                .tick(settings.app_lock, lock_after, active, Instant::now())
            {
                info!("Clipsa locked after inactivity");
                emit_lock_state(&app, &state);
            }
        })
        .expect("failed to spawn lock timer thread");
}

fn monitoring_label(settings: &AppSettings) -> &'static str {
    if settings.enable_monitoring {
        "Pause monitoring"
    } else {
        "Resume monitoring"
    }
}

/// Tray items whose text or shortcut follows the settings.
struct TrayItems {
    monitoring: MenuItem<Wry>,
    picker: MenuItem<Wry>,
}

impl TrayItems {
    /// Show the global hotkey next to "Quick picker" (display only: the menu
    /// doesn't register it). Not shown when it is off or the menu can't parse it.
    fn show_hotkey(&self, hotkey: &str) {
        let shown = (!hotkey.is_empty()).then_some(hotkey);
        if self.picker.set_accelerator(shown).is_err() {
            let _ = self.picker.set_accelerator(None::<&str>);
        }
    }
}

fn build_tray(app: &AppHandle, settings: &AppSettings) -> ClipsaResult<TrayItems> {
    let tauri_err = |e: tauri::Error| ClipsaError::ApplicationStartup(e.to_string());
    let item = |id: &str, text: &str| {
        MenuItem::with_id(app, id, text, true, None::<&str>).map_err(tauri_err)
    };
    let separator = || PredefinedMenuItem::separator(app).map_err(tauri_err);

    let open = item("open", "Open Clipsa")?;
    let picker = item("picker", "Quick picker")?;
    let monitoring = item("monitoring", monitoring_label(settings))?;
    let settings_item = item("settings", "Settings…")?;
    #[cfg(feature = "self-update")]
    let check_updates = item("check_updates", "Check for Updates…")?;
    let quit = item("quit", "Quit Clipsa")?;
    let (separator1, separator2, separator3) = (separator()?, separator()?, separator()?);

    let mut items: Vec<&dyn IsMenuItem<Wry>> = vec![
        &open,
        &picker,
        &separator1,
        &monitoring,
        &separator2,
        &settings_item,
    ];
    #[cfg(feature = "self-update")]
    items.push(&check_updates);
    items.extend([&separator3 as &dyn IsMenuItem<Wry>, &quit]);
    let menu = Menu::with_items(app, &items).map_err(tauri_err)?;

    let mut tray = TrayIconBuilder::with_id("clipsa")
        .tooltip("Clipsa")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main_window(app),
            "picker" => show_picker(app),
            "monitoring" => {
                let state = app.state::<AppState>();
                let enabled = state.settings().enable_monitoring;
                if let Err(e) = apply_settings_patch(
                    app,
                    &state,
                    serde_json::json!({ "enable_monitoring": !enabled }),
                ) {
                    error!("Toggling monitoring failed: {e}");
                }
            }
            "settings" => open_main_page(app, "/settings"),
            #[cfg(feature = "self-update")]
            "check_updates" => updater::show(app, true),
            "quit" => app.exit(0),
            _ => {}
        });
    // macOS menu bar: monochrome template image that follows the bar's
    // light/dark appearance. Elsewhere the full-colour app icon.
    #[cfg(target_os = "macos")]
    {
        let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray-template.png"))
            .map_err(tauri_err)?;
        tray = tray.icon(icon).icon_as_template(true);
    }
    #[cfg(not(target_os = "macos"))]
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app).map_err(tauri_err)?;

    let items = TrayItems { monitoring, picker };
    items.show_hotkey(&settings.global_hotkey);
    Ok(items)
}

/// macOS app menu ("Clipsa" next to the Apple menu): Tauri's default menu plus
/// "Settings… ⌘," after "About Clipsa", where Mac apps keep it. The tray's menu
/// handler receives this item's events too.
#[cfg(target_os = "macos")]
fn install_app_menu(app: &AppHandle) -> tauri::Result<()> {
    use tauri::menu::MenuItemKind;
    let menu = Menu::default(app)?;
    if let Some(MenuItemKind::Submenu(app_menu)) = menu.items()?.into_iter().next() {
        let settings = MenuItem::with_id(app, "settings", "Settings…", true, Some("CmdOrCtrl+,"))?;
        // Default: About, separator, Services, …; Settings goes in its own group.
        app_menu.insert(&settings, 2)?;
        app_menu.insert(&PredefinedMenuItem::separator(app)?, 3)?;
    }
    app.set_menu(menu)?;
    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn show_picker(app: &AppHandle) {
    if let Some(picker) = app.get_webview_window(PICKER_WINDOW) {
        let _ = picker.center();
        let _ = picker.show();
        let _ = picker.set_focus();
        let _ = app.emit_to(PICKER_WINDOW, EVENT_PICKER_SHOWN, ());
    }
}

fn toggle_picker(app: &AppHandle) {
    match app.get_webview_window(PICKER_WINDOW) {
        Some(picker) if picker.is_visible().unwrap_or(false) => {
            let _ = picker.hide();
        }
        _ => show_picker(app),
    }
}

/// Replace the registered global shortcut. An empty string disables it.
fn register_hotkey(app: &AppHandle, hotkey: &str) -> ClipsaResult<()> {
    let shortcut =
        if hotkey.is_empty() {
            None
        } else {
            Some(hotkey.parse::<Shortcut>().map_err(|e| {
                ClipsaError::InvalidSetting(format!("invalid hotkey `{hotkey}`: {e}"))
            })?)
        };
    let manager = app.global_shortcut();
    manager
        .unregister_all()
        .map_err(|e| ClipsaError::Platform(e.to_string()))?;
    if let Some(shortcut) = shortcut {
        manager.register(shortcut).map_err(|e| {
            ClipsaError::InvalidSetting(format!("hotkey `{hotkey}` could not be registered: {e}"))
        })?;
    }
    Ok(())
}

/// The direct edition's login entry. Its name stays the product name that
/// `tauri-plugin-autostart` used up to 0.1.1, so existing entries are found.
/// Debug builds use their own entry, as they use their own data directory, so
/// running `pnpm tauri dev` never removes or redirects the installed app's.
#[cfg(not(feature = "app-store"))]
fn autostart_entry(app: &AppHandle) -> ClipsaResult<platform::autostart::Autostart> {
    #[cfg(target_os = "linux")]
    let appimage = app.env().appimage.map(PathBuf::from);
    #[cfg(not(target_os = "linux"))]
    let appimage: Option<PathBuf> = None;
    let name = &app.package_info().name;
    let name = if cfg!(debug_assertions) {
        format!("{name} Dev")
    } else {
        name.clone()
    };
    platform::autostart::Autostart::new(
        &name,
        &app.config().identifier,
        LAUNCHED_AT_LOGIN_ARG,
        appimage.as_deref(),
    )
}

#[cfg(not(feature = "app-store"))]
fn set_autostart(app: &AppHandle, enabled: bool) -> ClipsaResult<()> {
    app.try_state::<platform::autostart::Autostart>()
        .ok_or_else(|| ClipsaError::Platform("Launch at login is unavailable".into()))?
        .set(enabled)
}

#[cfg(feature = "app-store")]
fn set_autostart(_app: &AppHandle, enabled: bool) -> ClipsaResult<()> {
    platform::login_item::set(enabled)
}

/// Validate, apply side effects, persist and broadcast a settings change.
/// On failure nothing is persisted and side effects are rolled back.
fn apply_settings_patch(
    app: &AppHandle,
    state: &AppState,
    patch: serde_json::Value,
) -> ClipsaResult<AppSettings> {
    let _serialised = state
        .settings_update
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    let old = state.settings();
    let new = old.merged(patch)?;

    // Turning the lock on proves the OS prompt works for this user first, so
    // nobody can lock themselves out by enabling it.
    if new.app_lock && !old.app_lock {
        state
            .lock
            .authenticate(&state.platform, "turn on the Clipsa app lock")?;
    }

    let hotkey_changed = new.global_hotkey != old.global_hotkey;
    if hotkey_changed {
        if let Err(e) = register_hotkey(app, &new.global_hotkey) {
            let _ = register_hotkey(app, &old.global_hotkey);
            return Err(e);
        }
    }
    let rollback_hotkey = || {
        if hotkey_changed {
            let _ = register_hotkey(app, &old.global_hotkey);
        }
    };
    if new.launch_at_login != old.launch_at_login {
        if let Err(e) = set_autostart(app, new.launch_at_login) {
            rollback_hotkey();
            return Err(e);
        }
    }
    if let Err(e) = state.storage.save_settings(&new) {
        rollback_hotkey();
        if new.launch_at_login != old.launch_at_login {
            let _ = set_autostart(app, old.launch_at_login);
        }
        return Err(e);
    }
    *state.settings.write().unwrap_or_else(|p| p.into_inner()) = new.clone();

    let _ = state.tray.monitoring.set_text(monitoring_label(&new));
    if hotkey_changed {
        state.tray.show_hotkey(&new.global_hotkey);
    }
    if new.max_items != old.max_items || new.auto_cleanup_days != old.auto_cleanup_days {
        state.service.enforce_limits()?;
    }
    let _ = app.emit(EVENT_SETTINGS_CHANGED, &new);
    Ok(new)
}

// Tauri commands. `async` runs them off the main thread.

type CommandResult<T> = Result<T, String>;

fn to_command<T>(result: ClipsaResult<T>) -> CommandResult<T> {
    result.map_err(|e| e.to_string())
}

impl AppState {
    /// The history service, unless Clipsa is locked.
    fn unlocked(&self) -> ClipsaResult<&ClipboardService> {
        self.lock.ensure_unlocked()?;
        Ok(&self.service)
    }
}

#[tauri::command(async, rename_all = "snake_case")]
fn get_history(
    state: tauri::State<'_, AppState>,
    offset: Option<u64>,
    limit: Option<u64>,
    search: Option<String>,
    content_type: Option<ContentType>,
    pinned_only: Option<bool>,
    pinned_first: Option<bool>,
) -> CommandResult<HistoryPage> {
    to_command(state.unlocked().and_then(|service| {
        service.history(HistoryQuery {
            offset,
            limit,
            search,
            content_type,
            pinned_only: pinned_only.unwrap_or(false),
            pinned_first: pinned_first.unwrap_or(false),
        })
    }))
}

#[tauri::command(async, rename_all = "snake_case")]
fn get_item_content(
    state: tauri::State<'_, AppState>,
    id: String,
) -> CommandResult<Option<String>> {
    to_command(state.unlocked().and_then(|service| service.item_text(&id)))
}

#[tauri::command(async, rename_all = "snake_case")]
fn copy_item(
    state: tauri::State<'_, AppState>,
    id: String,
    plain_text: Option<bool>,
) -> CommandResult<()> {
    to_command(
        state
            .unlocked()
            .and_then(|service| service.copy_item(&id, plain_text.unwrap_or(false))),
    )
}

#[cfg(not(feature = "app-store"))]
#[tauri::command(async, rename_all = "snake_case")]
fn paste_item(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
    plain_text: Option<bool>,
) -> CommandResult<()> {
    let service = to_command(state.unlocked())?;
    if state.platform.permissions().accessibility == platform::PermissionState::Denied {
        return Err(
            "Pasting needs accessibility permission (System Settings → Privacy & Security → Accessibility)"
                .into(),
        );
    }
    to_command(service.copy_item(&id, plain_text.unwrap_or(false)))?;
    if let Some(picker) = app.get_webview_window(PICKER_WINDOW) {
        let _ = picker.hide();
    }
    // Hand focus back to the application the user was in before the picker.
    #[cfg(target_os = "macos")]
    let _ = app.hide();
    std::thread::sleep(Duration::from_millis(150));
    // enigo looks up the key for "v" in the current keyboard layout through
    // macOS Text Input Sources, which may only be called on the main thread:
    // otherwise macOS can trap (SIGTRAP), e.g. after a layout switch or wake.
    // This command runs on a worker thread, so only the key presses move.
    let platform = Arc::clone(&state.platform);
    let (done, result) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = done.send(platform.simulate_paste());
    })
    .map_err(|e| format!("Paste failed: {e}"))?;
    let pasted = result
        .recv()
        .map_err(|_| "Paste failed: the main thread dropped the request".to_string())?;
    to_command(pasted)
}

/// The App Store sandbox does not allow sending keystrokes to other apps, so
/// this edition never pastes: `get_build_info().auto_paste` is false and the
/// picker uses `copy_item` instead. Rejects without touching the clipboard.
#[cfg(feature = "app-store")]
#[tauri::command(rename_all = "snake_case")]
fn paste_item(id: String, plain_text: Option<bool>) -> CommandResult<()> {
    let _ = (id, plain_text);
    Err("Pasting into other apps is not available in the App Store edition. Copy the item and press ⌘V.".into())
}

#[tauri::command(async, rename_all = "snake_case")]
fn delete_item(state: tauri::State<'_, AppState>, id: String) -> CommandResult<()> {
    to_command(
        state
            .unlocked()
            .and_then(|service| service.delete_item(&id)),
    )
}

#[tauri::command(async, rename_all = "snake_case")]
fn set_pinned(
    state: tauri::State<'_, AppState>,
    id: String,
    pinned: bool,
) -> CommandResult<ItemView> {
    to_command(
        state
            .unlocked()
            .and_then(|service| service.set_pinned(&id, pinned)),
    )
}

#[tauri::command(async, rename_all = "snake_case")]
fn create_snippet(
    state: tauri::State<'_, AppState>,
    text: String,
    title: Option<String>,
) -> CommandResult<ItemView> {
    to_command(
        state
            .unlocked()
            .and_then(|service| service.create_snippet(text, title)),
    )
}

#[tauri::command(async, rename_all = "snake_case")]
fn update_item(
    state: tauri::State<'_, AppState>,
    id: String,
    title: Option<String>,
    text: Option<String>,
) -> CommandResult<ItemView> {
    to_command(
        state
            .unlocked()
            .and_then(|service| service.update_item(&id, title, text)),
    )
}

#[tauri::command(async, rename_all = "snake_case")]
fn clear_history(state: tauri::State<'_, AppState>) -> CommandResult<()> {
    to_command(state.unlocked().and_then(|service| service.clear_history()))
}

#[tauri::command(async, rename_all = "snake_case")]
fn get_statistics(state: tauri::State<'_, AppState>) -> CommandResult<StatsView> {
    to_command(state.unlocked().and_then(|service| service.statistics()))
}

#[tauri::command(async, rename_all = "snake_case")]
fn get_settings(state: tauri::State<'_, AppState>) -> AppSettings {
    state.settings()
}

#[tauri::command(async, rename_all = "snake_case")]
fn update_settings(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    settings: serde_json::Value,
) -> CommandResult<AppSettings> {
    to_command(
        state
            .lock
            .ensure_unlocked()
            .and_then(|_| apply_settings_patch(&app, &state, settings)),
    )
}

#[tauri::command(async, rename_all = "snake_case")]
fn check_permissions(state: tauri::State<'_, AppState>) -> Permissions {
    state.platform.permissions()
}

#[tauri::command(async, rename_all = "snake_case")]
fn request_permissions(state: tauri::State<'_, AppState>) -> Permissions {
    state.platform.request_permissions()
}

#[tauri::command(rename_all = "snake_case")]
fn hide_picker(app: AppHandle) {
    if let Some(picker) = app.get_webview_window(PICKER_WINDOW) {
        let _ = picker.hide();
    }
}

/// Whether `path` is a page of the main window that other windows may open.
fn is_main_window_path(path: &str) -> bool {
    let route = path.split(['?', '#']).next().unwrap_or_default();
    matches!(route, "/" | "/settings")
}

/// Close the picker and bring up the main window on `path` (e.g. `/` or
/// `/settings?section=general`).
#[tauri::command(rename_all = "snake_case")]
fn open_main_window(app: AppHandle, path: String) -> CommandResult<()> {
    if !is_main_window_path(&path) {
        return Err(format!("Unknown page: {path}"));
    }
    open_main_page(&app, &path);
    Ok(())
}

/// Used by the picker, the tray and the app menu.
fn open_main_page(app: &AppHandle, path: &str) {
    if let Some(picker) = app.get_webview_window(PICKER_WINDOW) {
        let _ = picker.hide();
    }
    show_main_window(app);
    let _ = app.emit_to(MAIN_WINDOW, EVENT_MAIN_NAVIGATE, path);
}

#[tauri::command]
fn get_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Which distribution this binary is, so the UI can hide what doesn't apply.
#[derive(serde::Serialize)]
struct BuildInfo {
    /// "direct" (GitHub/Homebrew) or "app_store".
    edition: &'static str,
    /// Built-in updater present.
    self_update: bool,
    /// The picker can paste into the previous app (not allowed in the App Store sandbox).
    auto_paste: bool,
}

#[tauri::command]
fn get_build_info() -> BuildInfo {
    BuildInfo {
        edition: if cfg!(feature = "app-store") {
            "app_store"
        } else {
            "direct"
        },
        self_update: cfg!(feature = "self-update"),
        auto_paste: !cfg!(feature = "app-store"),
    }
}

#[tauri::command(async, rename_all = "snake_case")]
fn get_lock_state(state: tauri::State<'_, AppState>) -> LockState {
    state.lock_state()
}

#[tauri::command(async, rename_all = "snake_case")]
fn unlock(app: AppHandle, state: tauri::State<'_, AppState>) -> CommandResult<LockState> {
    to_command(state.lock.unlock(&state.platform))?;
    Ok(emit_lock_state(&app, &state))
}

#[tauri::command(async, rename_all = "snake_case")]
fn lock_now(app: AppHandle, state: tauri::State<'_, AppState>) -> LockState {
    if state.settings().app_lock && state.platform.auth_supported() {
        state.lock.lock();
    }
    emit_lock_state(&app, &state)
}

#[cfg(test)]
mod tests {
    use super::is_main_window_path;

    #[test]
    fn only_main_window_pages_can_be_opened() {
        for ok in [
            "/",
            "/?view=pinned",
            "/settings",
            "/settings?section=privacy",
            "/#top",
        ] {
            assert!(is_main_window_path(ok), "{ok}");
        }
        for bad in [
            "",
            "/picker",
            "/updater",
            "https://example.com",
            "//evil.example/",
            "/settings/../picker",
        ] {
            assert!(!is_main_window_path(bad), "{bad}");
        }
    }
}
