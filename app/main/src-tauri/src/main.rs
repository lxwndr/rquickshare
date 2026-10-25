#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

#[macro_use]
extern crate log;

use std::sync::{Arc, Mutex};

use rqs_lib::channel::{ChannelDirection, ChannelMessage};
use rqs_lib::{EndpointInfo, SendInfo, State, Visibility, RQS};
use store::get_startminimized;
use tauri::image::Image;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Listener, Manager, Window, WindowEvent,
};
use tauri_plugin_autostart::MacosLauncher;
use tokio::sync::{broadcast, mpsc, watch};

use crate::logger::set_up_logging;
use crate::notification::{send_request_notification, send_temporarily_notification};
use crate::store::{
    get_download_path, get_port, get_realclose, get_visibility, init_default, set_visibility,
};

mod cmds;
mod logger;
mod notification;
mod store;

pub struct AppState {
    pub message_sender: broadcast::Sender<ChannelMessage>,
    pub dch_sender: broadcast::Sender<EndpointInfo>,
    pub visibility_sender: Arc<Mutex<watch::Sender<Visibility>>>,
    pub sender_file: mpsc::Sender<SendInfo>,
    pub ble_receiver: broadcast::Receiver<()>,
    pub rqs: Mutex<RQS>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Define tauri async runtime to be tokio
    tauri::async_runtime::set(tokio::runtime::Handle::current());

    // Build and run Tauri app
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            trace!("tauri_plugin_single_instance: instance already running");
            open_main_window(app);

            // Forward local paths passed by the file manager to the running instance.
            let files: Vec<String> = argv
                .iter()
                .skip(1)
                .filter(|arg| std::path::Path::new(arg).exists())
                .cloned()
                .collect();
            if !files.is_empty() {
                if let Err(error) = app.emit("send-files-requested", files) {
                    error!("Could not forward file-manager selection: {error}");
                }
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            cmds::change_download_path,
            cmds::change_visibility,
            cmds::start_discovery,
            cmds::stop_discovery,
            cmds::get_hostname,
            cmds::send_payload,
            cmds::send_to_rs,
            cmds::get_cli_files,
        ])
        .setup(|app| {
            set_up_logging(app.app_handle())?;
            debug!("Starting setup of RQuickShare app");
            init_default(app.app_handle());

            let name = MenuItemBuilder::new("RQuickShare")
                .enabled(false)
                .build(app)?;
            let show = MenuItemBuilder::with_id("show", "Exibir").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Sair").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&name)
                .separator()
                .items(&[&show, &quit])
                .build()?;

            // Use the dedicated high-resolution tray icon on every platform.
            let icon = tray_icon_for_current_theme();

            let tray = TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "show" => {
                        trace!("tray_show");
                        open_main_window(app);
                    }
                    "quit" => {
                        trace!("tray_quit");
                        kill_app(app.app_handle());
                    }
                    _ => (),
                })
                .build(app)?;

            #[cfg(target_os = "macos")]
            let _ = tray.set_icon_as_template(true);

            // Follow the theme actually applied by the frontend, not the desktop theme.
            let tray_for_theme = tray.clone();
            app.listen("rquickshare-theme-changed", move |event| {
                let dark = event.payload() == "true";
                if let Err(error) = tray_for_theme.set_icon(Some(tray_icon_for_theme(dark))) {
                    warn!("Could not update tray icon for app theme: {error}");
                }
            });

            let visibility = get_visibility(app.app_handle());
            let port_number = get_port(app.app_handle());
            let download_path = get_download_path(app.app_handle());

            let app_handle = app.app_handle().clone();
            tokio::task::block_in_place(|| {
                tauri::async_runtime::block_on(async move {
                    trace!("Beginning of RQS start");
                    let mut rqs = RQS::new(visibility, port_number, download_path);
                    let (sender_file, ble_receiver) = rqs.run().await.unwrap();
                    app_handle.manage(AppState {
                        message_sender: rqs.message_sender.clone(),
                        dch_sender: broadcast::channel(10).0,
                        visibility_sender: rqs.visibility_sender.clone(),
                        sender_file,
                        ble_receiver,
                        rqs: Mutex::new(rqs),
                    });
                });
            });

            spawn_receiver_tasks(app.app_handle());
            Ok(())
        })
        .on_window_event(handle_window_event)
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| match event {
            tauri::RunEvent::Ready { .. } => {
                trace!("RunEvent::Ready");
                if get_startminimized(app_handle) {
                    #[cfg(not(target_os = "macos"))]
                    app_handle.get_webview_window("main").unwrap().hide().unwrap();
                    #[cfg(target_os = "macos")]
                    app_handle.hide().unwrap();
                }
            }
            tauri::RunEvent::ExitRequested { code, .. } => {
                trace!("RunEvent::ExitRequested");
                if code != Some(-1) {
                    kill_app(app_handle);
                }
            }
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => {
                trace!("RunEvent::Reopen");
                open_main_window(app_handle);
            }
            _ => {}
        });

    info!("Application stopped");
    Ok(())
}


fn tray_icon_for_theme(dark_theme: bool) -> Image<'static> {
    let bytes: &'static [u8] = if dark_theme {
        include_bytes!("../icons/tray-light.png")
    } else {
        include_bytes!("../icons/tray-dark.png")
    };
    Image::from_bytes(bytes).expect("generated tray icon must be a valid PNG")
}

fn tray_icon_for_current_theme() -> Image<'static> {
    tray_icon_for_theme(is_dark_desktop_theme())
}

fn spawn_receiver_tasks(app_handle: &AppHandle) {
    let capp_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        let state: tauri::State<'_, AppState> = capp_handle.state();
        let mut receiver = state.message_sender.subscribe();
        loop {
            match receiver.recv().await {
                Ok(info) => {
                    if info.state.as_ref().unwrap_or(&State::Initial) == &State::WaitingForUserConsent {
                        let name = info.meta.as_ref().and_then(|meta| meta.source.as_ref())
                            .map(|source| source.name.clone()).unwrap_or_else(|| "Unknown".to_string());
                        send_request_notification(name, info.id.clone(), &capp_handle);
                    }
                    rs2js_channelmessage(info, &capp_handle);
                }
                Err(e) => error!("RecvError: message_sender: {e}"),
            }
        }
    });
    let capp_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        let state: tauri::State<'_, AppState> = capp_handle.state();
        let mut dch_receiver = state.dch_sender.subscribe();
        loop {
            match dch_receiver.recv().await {
                Ok(info) => rs2js_endpointinfo(info, &capp_handle),
                Err(e) => error!("RecvError: dch_sender: {e}"),
            }
        }
    });
    let capp_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        let state: tauri::State<'_, AppState> = capp_handle.state();
        let mut visibility_receiver = state.visibility_sender.lock().unwrap().subscribe();
        loop {
            match visibility_receiver.changed().await {
                Ok(_) => { let v = visibility_receiver.borrow_and_update(); let _ = set_visibility(&capp_handle, *v); }
                Err(e) => error!("RecvError: visibility_receiver: {e}"),
            }
        }
    });
    let capp_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        let state: tauri::State<'_, AppState> = capp_handle.state();
        let mut ble_receiver = state.ble_receiver.resubscribe();
        let mut last_sent = std::time::Instant::now() - std::time::Duration::from_secs(120);
        loop {
            match ble_receiver.recv().await {
                Ok(_) => {
                    let v = get_visibility(&capp_handle);
                    trace!("Tauri: ble received: {:?}", v);
                    if v == Visibility::Invisible && last_sent.elapsed() >= std::time::Duration::from_secs(120) {
                        send_temporarily_notification(&capp_handle);
                        last_sent = std::time::Instant::now();
                    }
                }
                Err(e) => error!("RecvError: ble_receiver: {e}"),
            }
        }
    });
}

fn handle_window_event(w: &Window, event: &WindowEvent) {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        if get_realclose(w.app_handle()) { trace!("handle_window_event: real close"); return; }
        trace!("handle_window_event: prevent close");
        w.hide().unwrap();
        api.prevent_close();
    }
}

fn rs2js_channelmessage(message: ChannelMessage, manager: &AppHandle) {
    if message.direction == ChannelDirection::FrontToLib { return; }
    info!("rs2js_channelmessage: {:?}", &message);
    manager.emit("rs2js_channelmessage", &message).unwrap();
}

fn rs2js_endpointinfo(message: EndpointInfo, manager: &AppHandle) {
    info!("rs2js_endpointinfo: {:?}", &message);
    manager.emit("rs2js_endpointinfo", &message).unwrap();
}

fn open_main_window(app_handle: &AppHandle) {
    if let Some(webview_window) = app_handle.get_webview_window("main") {
        let _ = webview_window.show();
        let _ = webview_window.set_focus();
        return;
    }
    warn!("open_main_window: no main window found");
}

fn kill_app(app_handle: &AppHandle) {
    let state: tauri::State<'_, AppState> = app_handle.state();
    tokio::task::block_in_place(|| {
        #[allow(clippy::await_holding_lock)]
        tauri::async_runtime::block_on(async move { let _ = state.rqs.lock().unwrap().stop().await; });
    });
    app_handle.exit(-1);
}
