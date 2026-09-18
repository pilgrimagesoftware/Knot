use super::*;
pub(crate) fn start_mcp_server(agents: Arc<Mutex<knot_agents::AgentStore>>,
                               settings: knot_core::Settings, notifier: Arc<QueuedNotifier>,
                               messages: Arc<Mutex<knot_messaging::MessageStore>>,
                               awaiting_input: AwaitingInputQueue)
                               -> tokio::sync::oneshot::Sender<()> {
    let (stop, stop_rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(err) => {
                eprintln!("failed to start MCP server runtime: {err}");
                return;
            }
        };
        runtime.block_on(async move {
                   if !settings.mcp_server_enabled {
                       return;
                   }

                   let (discovery, repos_rx) = knot_discovery::Discovery::new();
                   if !settings.source_base_folder.is_empty()
                && let Err(err) =
                    discovery.set_source_folder(Some(PathBuf::from(&settings.source_base_folder)))
            {
                eprintln!("failed to watch source folder: {err}");
            }

                   let catalog = Arc::new(
                knot_mcp_tools::McpToolCatalog::new(agents, repos_rx, notifier)
                    .with_message_store(messages)
                    .with_awaiting_input_queue(awaiting_input)
                    .with_settings(settings.clone()),
            );
                   catalog.set_bench_agents(settings.bench_agents.clone());

                   let agents_snapshot: knot_mcp::AgentsSnapshotFn = {
                       let catalog = catalog.clone();
                       Arc::new(move || catalog.agents_snapshot())
                   };
                   let hook_handler = catalog.clone();
                   let mut server =
                       knot_mcp::McpServer::new(settings.mcp_server_port,
                                                catalog as Arc<dyn ToolCatalog>,
                                                agents_snapshot).with_hook_handler(hook_handler);
                   if let Err(err) = server.start().await {
                       eprintln!("failed to start MCP server: {err}");
                       return;
                   }

                   tokio::select! {
                       _ = stop_rx => {}
                       _ = std::future::pending::<()>() => {}
                   }
                   server.stop();
                   drop(discovery);
               });
    });
    stop
}

actions!(knot_app,
         [Quit,
          HideApp,
          HideOthers,
          ShowAllWindows,
          AboutKnot,
          OpenSettings,
          PanelPermissionAllow,
          PanelPermissionDeny,
          PanelOpenPermissionSelector]);

pub(crate) fn quit(_: &Quit, cx: &mut App) {
    cx.quit();
}

pub(crate) fn hide_app(_: &HideApp, cx: &mut App) {
    cx.hide();
}

pub(crate) fn hide_others(_: &HideOthers, cx: &mut App) {
    cx.hide_other_apps();
}

pub(crate) fn show_all_windows(_: &ShowAllWindows, cx: &mut App) {
    cx.activate(true);
}

pub(crate) fn about_knot(_: &AboutKnot, cx: &mut App) {
    if let Some(window) = cx.active_window() {
        let _ = window.update(cx, |_, window, cx| {
                          window.open_alert_dialog(cx, |alert, _, _| {
                                    alert
                    .title("About Knot")
                    .description("Knot is a workspace for coordinating coding agents.")
                    .show_cancel(false)
                                });
                      });
    }
}

pub(crate) fn set_app_menus(cx: &mut App) {
    cx.set_menus([
        Menu::new("Knot").items([
            MenuItem::action("About Knot", AboutKnot),
            MenuItem::separator(),
            MenuItem::action("Settings…", OpenSettings),
            MenuItem::separator(),
            MenuItem::os_submenu("Services", SystemMenuType::Services),
            MenuItem::separator(),
            MenuItem::action("Hide Knot", HideApp),
            MenuItem::action("Hide Others", HideOthers),
            MenuItem::action("Show All", ShowAllWindows),
            MenuItem::separator(),
            MenuItem::action("Quit Knot", Quit),
        ]),
        Menu::new("File").items([
            MenuItem::action("New Workspace", gpui_kit::NoAction).disabled(true),
            MenuItem::separator(),
            MenuItem::action("Close Window", gpui_kit::NoAction).disabled(true),
        ]),
        Menu::new("Edit").items([
            MenuItem::action("Undo", gpui_kit::NoAction).disabled(true),
            MenuItem::action("Redo", gpui_kit::NoAction).disabled(true),
            MenuItem::separator(),
            MenuItem::action("Cut", gpui_kit::NoAction).disabled(true),
            MenuItem::action("Copy", gpui_kit::NoAction).disabled(true),
            MenuItem::action("Paste", gpui_kit::NoAction).disabled(true),
        ]),
        Menu::new("View")
            .items([MenuItem::action("Enter Full Screen", gpui_kit::NoAction).disabled(true)]),
        Menu::new("Window").items([
            MenuItem::action("Minimize", gpui_kit::NoAction).disabled(true),
            MenuItem::action("Zoom", gpui_kit::NoAction).disabled(true),
        ]),
        Menu::new("Help").items([MenuItem::action("Knot Help", gpui_kit::NoAction).disabled(true)]),
    ]);
}

pub(crate) fn run() {
    let mut settings = knot_core::Settings::load().unwrap_or_default();
    if let Err(err) = settings.init_source_folder() {
        eprintln!("failed to initialize source folder: {err}");
    }
    if let Err(err) = settings.install_default_personas() {
        eprintln!("failed to install default personas: {err}");
    }
    let store = Arc::new(Mutex::new(build_agent_store(&settings)));
    let notifier = Arc::new(QueuedNotifier::new());
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let awaiting_input = Arc::new(Mutex::new(Vec::new()));
    let mcp_stop = start_mcp_server(Arc::clone(&store),
                                    settings.clone(),
                                    Arc::clone(&notifier),
                                    Arc::clone(&messages),
                                    Arc::clone(&awaiting_input));

    gpui_kit::application()
                           // `Assets` only embeds gpui-component's own curated icon subset; our
                           // settings-window icon buttons (folder-open/pencil/trash/x/plus/copy)
                           // aren't in it, so `Icon::path(...)` silently resolved to nothing and
                           // rendered invisible. `AllAssets` embeds the complete Lucide catalog.
                           .with_assets(gpui_kit::assets::AllAssets)
                           .run(move |cx| {
                               // Before `set_app_menus`: AppKit labels the
                               // application menu from the process name.
                               app_support::set_process_name(&knot_core::l10n::t("app.name"));
                               gpui_kit::init(cx);
                               Theme::change(cx.window_appearance(), None, cx);
                               apply_visual_identity(&settings, cx);

                               cx.on_action(quit);
                               cx.on_action(about_knot);
                               cx.on_action(hide_app);
                               cx.on_action(hide_others);
                               cx.on_action(show_all_windows);
                               // The standard macOS application-menu
                               // shortcuts. A `MenuItem::action` only shows a
                               // shortcut next to its label if the action has
                               // a binding, so without these the menu read as
                               // if Knot had none.
                               cx.bind_keys([KeyBinding::new("cmd-q", Quit, None),
                                             KeyBinding::new("cmd-,", OpenSettings, None),
                                             KeyBinding::new("cmd-h", HideApp, None),
                                             KeyBinding::new("cmd-alt-h", HideOthers, None)]);
                               cx.bind_keys([KeyBinding::new("cmd-shift-a",
                                                             PanelPermissionAllow,
                                                             None),
                                             KeyBinding::new("cmd-shift-d",
                                                             PanelPermissionDeny,
                                                             None),
                                             KeyBinding::new("cmd-shift-p",
                                                             PanelOpenPermissionSelector,
                                                             None)]);
                               let settings_window: Rc<RefCell<Option<AnyWindowHandle>>> =
                                   Rc::new(RefCell::new(None));
                               {
                                   let settings_window = Rc::clone(&settings_window);
                                   let settings = settings.clone();
                                   cx.on_action(move |_: &OpenSettings, cx| {
                                         open_settings_window(&settings_window,
                                                              settings.clone(),
                                                              cx);
                                     });
                               }
                               set_app_menus(cx);

                               cx.on_system_notification_response(|response, cx| {
                                     if notification_response_agent_id(&response).is_some() {
                                         cx.activate(true);
                                     }
                                 });

                               let options = manager_window_options(cx);
                               cx.open_window(options, |window, cx| {
                                     let name_input =
                    cx.new(|cx| InputState::new(window, cx).placeholder("Workspace name"));
                                     let view =
                                         cx.new(|_| WorkspaceManager { store:
                                                                           Arc::clone(&store),
                                                                       settings:
                                                                           settings.clone(),
                                                                       name_input,
                                                                       editing_id: None,
                                                                       workspace_dialog_id:
                                                                           None,
                                                                       show_workspace_dialog:
                                                                           false,
                                                                       delete_workspace_id:
                                                                           None,
                                                                       error: None,
                                                                       _mcp_stop:
                                                                           Some(mcp_stop) });
                                     cx.new(|cx| {
                                           Root::new(view, window, cx).bg(cx.theme().background)
                                       })
                                 })
                                 .expect("failed to open workspace manager");
                               // macOS launches a non-bundled binary without
                               // making it frontmost, so without this the
                               // window opens behind whatever was already on
                               // screen. `activate` is the app-level
                               // equivalent of ordering the window front.
                               cx.activate(true);
                           });
}
