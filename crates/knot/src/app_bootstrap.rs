use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::AnyWindowHandle;
use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::KeyBinding;
use gpui_kit::Menu;
use gpui_kit::MenuItem;
use gpui_kit::SystemMenuType;
use gpui_kit::actions;
use gpui_kit::base::input;
use gpui_kit::component::Root;
use gpui_kit::component::Theme;
use gpui_kit::component::input::InputEvent;
use gpui_kit::component::input::InputState;
use knot_mcp::ToolCatalog;
use knot_messaging::QueuedNotifier;
use parking_lot::Mutex;

use crate::about_window::register_about_action;
use crate::agent_menu::AgentMenuSnapshot;
use crate::agent_menu::AgentsMenuState;
use crate::agent_menu::agent_menu_key_bindings;
use crate::agent_menu::agents_menu;
use crate::app_state::build_agent_store;
use crate::app_state::notification_response_agent_id;
use crate::app_support;
use crate::app_support::Activation;
use crate::app_support::ActivationQueue;
use crate::app_support::AwaitingInput;
use crate::app_support::AwaitingInputQueue;
use crate::app_support::apply_visual_identity;
use crate::app_support::observe_system_appearance;
use crate::command_center::CommandCenterWindow;
use crate::import_window::register_import_action;
use crate::mcp_status;
use crate::mcp_status::McpServerStatus;
use crate::quit_guard;
use crate::settings_window::open_settings_window;
use crate::window_options::manager_window_options;
use crate::workspace_manager::WorkspaceManager;

/// Starts the MCP server under supervision, reporting the stop signal that
/// ends both and the status the rest of the application watches.
///
/// The server used to be started once here and never watched again: when
/// its serve task ended the port went quiet for the rest of the process's
/// life, and a failed bind ended the MCP subsystem for the session. The
/// supervisor owns that lifecycle now; this still owns the thread, the
/// runtime and the oneshot that stops it.
///
/// A server configuration has turned off leaves the status at
/// [`knot_mcp::ServerState::Disabled`]: nothing is bound and nothing is
/// supervised.
pub(crate) fn start_mcp_server(agents: Arc<Mutex<knot_agents::AgentStore>>,
                               settings: knot_core::Settings, notifier: Arc<QueuedNotifier>,
                               messages: Arc<Mutex<knot_messaging::MessageStore>>,
                               awaiting_input: AwaitingInputQueue, activation: ActivationQueue)
                               -> (tokio::sync::oneshot::Sender<()>, McpServerStatus) {
    let (stop, stop_rx) = tokio::sync::oneshot::channel();
    let status = McpServerStatus::new();
    let thread_status = status.clone();
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
                    .with_activation_queue(activation)
                    .with_settings(settings.clone()),
            );
                   catalog.set_bench_agents(settings.bench_agents.clone());

                   let agents_snapshot: knot_mcp::AgentsSnapshotFn = {
                       let catalog = catalog.clone();
                       Arc::new(move || catalog.agents_snapshot())
                   };
                   let hook_handler = catalog.clone();
                   let supervisor =
                       knot_mcp::Supervisor::new(settings.mcp_server_port,
                                                 catalog as Arc<dyn ToolCatalog>,
                                                 agents_snapshot).with_hook_handler(hook_handler);
                   // Subscribed before `run`, so no transition is missed -
                   // though a `watch` receiver would read the current value
                   // even if it were not.
                   let mirroring =
                       tokio::spawn(mcp_status::mirror_server_state(supervisor.state(),
                                                                    thread_status));
                   // Returns only on the stop signal, having released the
                   // port; everything else it retries.
                   supervisor.run(stop_rx).await;
                   mirroring.abort();
                   drop(discovery);
               });
    });
    (stop, status)
}

actions!(knot_app,
         [Quit,
          HideApp,
          HideOthers,
          ShowAllWindows,
          AboutKnot,
          OpenSettings,
          OpenImport,
          PanelPermissionAllow,
          PanelPermissionDeny,
          PanelOpenPermissionSelector]);

// UNWIRED: the standard-menu items the port has not implemented yet. They
// exist as named actions rather than `NoAction` only so each can carry its
// own key equivalent - a menu item's shortcut is looked up by action, so
// items sharing `NoAction` would all have to show the same one, and binding
// a key to `NoAction` *unbinds* that key everywhere (`Keymap::add_bindings`
// treats it as a disabling binding).
//
// Nothing registers a handler for any of these, so `is_action_available`
// answers false and AppKit draws the items disabled with their shortcut
// greyed beside them, which is how macOS presents a standard item an app
// does not currently offer. Wiring one is a matter of registering its
// handler on the window that owns the behavior; the binding is already
// here.
actions!(knot_app,
         [NewWorkspace, CloseWindow, MinimizeWindow, KnotHelp]);

// Enter Full Screen is deliberately absent. macOS adds its own item to the
// View menu when it does not find an equivalent one, and it judges
// equivalence by the action behind the item rather than by its label - so a
// Knot item it does not recognize was added beside rather than instead, and
// the menu showed the same command twice under the same key. cmd-ctrl-f is
// also a key the platform reserves, and `app-menu` forbids a Knot item from
// holding one: AppKit claims a menu key equivalent ahead of the window, so
// such an item takes the key rather than sharing it. The same rule moved Fork
// Agent off cmd-f.

// The Window menu's two openers. Unlike the items above these are wired, and
// enabled at all times: they are how a user gets back to a window, so an
// enablement rule that depended on a window being focused would disable them
// exactly when they are needed.
actions!(knot_app, [OpenCommandCenter, OpenWorkspaces]);

/// Every user-facing quit path lands here - the application menu's Quit
/// Knot item and the `cmd-q` binding both dispatch `Quit` - so the guard
/// only has to be applied once. See `quit_guard` for why the check cannot
/// live in `on_app_quit` instead.
pub(crate) fn quit(_: &Quit, cx: &mut App) {
    quit_guard::request_quit(cx);
}

/// Quits on Ctrl-C (or `kill`) from the launching terminal.
///
/// Under `cargo run` the signal appeared to be swallowed: the Cocoa run
/// loop keeps the process alive and nothing here handled it, so the only
/// way out was Cmd+Q or killing the process from another shell. A
/// terminal-launched process is expected to die on Ctrl-C, so this restores
/// that. It exits rather than routing through `cx.quit()` because the
/// handler runs off the main thread and cannot reach the app; the child
/// PTYs go with the process, and the MCP server's listener is closed by the
/// same exit.
#[cfg(unix)]
pub(crate) fn quit_on_terminal_signals() {
    use tokio::signal::unix::{SignalKind, signal};

    std::thread::spawn(|| {
        let runtime = match tokio::runtime::Builder::new_current_thread().enable_all()
                                                                         .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!("failed to start the signal runtime: {error}");
                return;
            }
        };
        runtime.block_on(async {
                   let (Ok(mut interrupt), Ok(mut terminate)) =
                       (signal(SignalKind::interrupt()), signal(SignalKind::terminate()))
                   else {
                       eprintln!("failed to install terminal signal handlers");
                       return;
                   };
                   tokio::select! {
                       _ = interrupt.recv() => {}
                       _ = terminate.recv() => {}
                   }
                   // 128 + SIGINT, the conventional shell exit code.
                   std::process::exit(130);
               });
    });
}

#[cfg(not(unix))]
pub(crate) fn quit_on_terminal_signals() {}

pub(crate) fn hide_app(_: &HideApp, cx: &mut App) {
    cx.hide();
}

pub(crate) fn hide_others(_: &HideOthers, cx: &mut App) {
    cx.hide_other_apps();
}

pub(crate) fn show_all_windows(_: &ShowAllWindows, cx: &mut App) {
    cx.activate(true);
}

/// Installs the menu bar.
///
/// Called again whenever `snapshot` changes, because a `Menu` is a static
/// snapshot: the Agents menu's Move to Workspace and Markdown Files
/// submenus cannot re-read the store on their own (`app-menu`). Everything
/// else in the bar is rebuilt identically, which is cheap and keeps the
/// whole bar described in one place.
pub(crate) fn set_app_menus(snapshot: &AgentMenuSnapshot, cx: &mut App) {
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
            MenuItem::action("New Workspace", NewWorkspace).disabled(true),
            MenuItem::separator(),
            MenuItem::action("Import…", OpenImport),
            MenuItem::separator(),
            MenuItem::action("Close Window", CloseWindow).disabled(true),
        ]),
        // The text actions gpui already defines and binds for a focused
        // input, rather than placeholders of our own. Reusing them is what
        // puts the standard shortcuts beside these labels, and it is also
        // the only safe way to get them: a placeholder action of ours bound
        // to cmd-c would out-rank the input's own binding - a context-less
        // binding ranks at the deepest context, and later bindings win ties
        // - and copying in a text field would stop working.
        //
        // They carry no `disabled`, because these five do work: AppKit asks
        // whether each action is available along the focused element's
        // dispatch path, so they enable with a text field focused and grey
        // out elsewhere. In the terminal pane, where nothing claims them,
        // the disabled items let cmd-c fall through to the pane's own
        // copy-selection handler.
        Menu::new("Edit").items([
            MenuItem::action("Undo", input::Undo),
            MenuItem::action("Redo", input::Redo),
            MenuItem::separator(),
            MenuItem::action("Cut", input::Cut),
            MenuItem::action("Copy", input::Copy),
            MenuItem::action("Paste", input::Paste),
        ]),
        // No items of Knot's own: macOS creates and populates this menu's
        // Enter Full Screen itself. See the note beside the `actions!` block.
        Menu::new("View").items([]),
        agents_menu(snapshot),
        // Knot's own items first, then a separator, then the list of open
        // windows macOS appends and maintains below them. Without the
        // separator a workspace called "Zoom" is indistinguishable from the
        // Zoom command, and these four shift down every time a window opens.
        // The two openers are their own group: they open windows, Minimize
        // and Zoom manipulate the focused one.
        Menu::new("Window").items([
            MenuItem::action(knot_core::l10n::t("menu.window.command_center"), OpenCommandCenter),
            MenuItem::action(knot_core::l10n::t("menu.window.workspaces"), OpenWorkspaces),
            MenuItem::separator(),
            MenuItem::action("Minimize", MinimizeWindow).disabled(true),
            // Zoom keeps `NoAction`: macOS gives it no key equivalent, so
            // it has no reason to be named.
            MenuItem::action("Zoom", gpui_kit::NoAction).disabled(true),
            MenuItem::separator(),
        ]),
        Menu::new("Help").items([MenuItem::action("Knot Help", KnotHelp).disabled(true)]),
    ]);
}

/// What [`open_workspace_manager`] needs to build the window, grouped so it
/// stays inside the workspace's argument-count convention.
struct WorkspaceManagerWindow {
    store:    Arc<Mutex<knot_agents::AgentStore>>,
    messages: Arc<Mutex<knot_messaging::MessageStore>>,
    settings: knot_core::Settings,
    /// Dropped with the window, which is what stops the MCP server when
    /// the last window closes.
    mcp_stop: tokio::sync::oneshot::Sender<()>,
}

/// The application-menu actions and their key bindings.
///
/// A `MenuItem::action` only shows a shortcut beside its label if the
/// action has a binding, so without these the menu read as if Knot had
/// none.
pub(crate) fn install_actions_and_keys(settings: &knot_core::Settings,
                                       store: Arc<Mutex<knot_agents::AgentStore>>, cx: &mut App) {
    cx.on_action(quit);
    // Holds its own window handle; see
    // `about_window::register_about_action`.
    register_about_action(settings.title_font_name.clone().into(), cx);
    // Holds its own window handle, and reloads the store when it opens; see
    // `import_window::register_import_action`.
    register_import_action(Arc::clone(&store), cx);
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
    // The rest of the shortcuts macOS expects on a standard menu item,
    // whether or not the item behind each is wired up yet - the menu bar
    // reads as an app with no keyboard at all without them. About Knot,
    // Show All and Zoom are absent on purpose: macOS gives those three no
    // key equivalent either. Cut, Copy, Paste, Undo and Redo are absent
    // because gpui already binds them for a focused input, and the Edit
    // menu points at those same actions rather than at ours.
    cx.bind_keys([KeyBinding::new("cmd-n", NewWorkspace, None),
                  KeyBinding::new("cmd-w", CloseWindow, None),
                  KeyBinding::new("cmd-m", MinimizeWindow, None),
                  KeyBinding::new("cmd-shift-/", KnotHelp, None)]);
    // The Window menu's openers. macOS reserves neither: cmd-0 is
    // conventionally "reset zoom" in a browser or an editor, and Knot has no
    // zoom level for it to reset.
    cx.bind_keys([KeyBinding::new("cmd-alt-0", OpenCommandCenter, None),
                  KeyBinding::new("cmd-0", OpenWorkspaces, None)]);
    // The Agents menu's own keys. No platform convention names these -
    // the items are Knot's - so they come from the Swift reference; see
    // `agent_menu::agent_menu_key_bindings`.
    cx.bind_keys(agent_menu_key_bindings());
    cx.bind_keys([KeyBinding::new("cmd-shift-a", PanelPermissionAllow, None),
                  KeyBinding::new("cmd-shift-d", PanelPermissionDeny, None),
                  KeyBinding::new("cmd-shift-p", PanelOpenPermissionSelector, None)]);
    let settings_window: Rc<RefCell<Option<AnyWindowHandle>>> = Rc::new(RefCell::new(None));
    {
        let settings_window = Rc::clone(&settings_window);
        let store = Arc::clone(&store);
        cx.on_action(move |_: &OpenSettings, cx| {
              // Reload from disk rather than reusing
              // a clone
              // captured at bootstrap: reopening the
              // window
              // with a stale snapshot would both
              // show old
              // values and overwrite a since-saved
              // change
              // the moment anything in the reopened
              // window
              // persists.
              let settings = knot_core::Settings::load().unwrap_or_default();
              open_settings_window(&settings_window, settings, Arc::clone(&store), cx);
          });
    }
}

/// Opens the workspace manager - the window the application starts in.
/// Wires the Window menu's two openers.
///
/// Registered here rather than in `install_actions_and_keys` because these
/// need the message store as well, and because what they do - raise a window
/// or open one - is this module's business rather than the keymap's.
///
/// Reopening the manager after it has been closed passes no `mcp_stop`; see
/// [`open_workspace_manager`].
fn register_window_actions(store: Arc<Mutex<knot_agents::AgentStore>>,
                           messages: Arc<Mutex<knot_messaging::MessageStore>>,
                           settings: knot_core::Settings, cx: &mut App) {
    {
        let store = Arc::clone(&store);
        let messages = Arc::clone(&messages);
        let settings = settings.clone();
        cx.on_action(move |_: &OpenCommandCenter, cx| {
              CommandCenterWindow::open(Arc::clone(&store),
                                        Arc::clone(&messages),
                                        settings.clone(),
                                        cx);
          });
    }
    cx.on_action(move |_: &OpenWorkspaces, cx| {
          crate::window_registry::activate_or_open(
              crate::window_registry::WindowKey::WorkspaceManager,
              cx,
              |cx| {
                  open_manager_window(Arc::clone(&store), Arc::clone(&messages), settings.clone(), None, cx)
              },
          );
      });
}

/// Opens the workspace manager, or raises it when one is already open.
///
/// One manager window, like the Command Center
/// (`openspec/specs/window-lifecycle`). Reopening after a close passes no
/// `mcp_stop`: the oneshot that keeps the MCP server alive went with the
/// window that held it, and this does not resurrect it - closing the manager
/// has always stopped the server, and that is a separate question from how
/// many manager windows there are.
fn open_workspace_manager(parts: WorkspaceManagerWindow, cx: &mut App) {
    let WorkspaceManagerWindow { store,
                                 messages,
                                 settings,
                                 mcp_stop, } = parts;
    crate::window_registry::activate_or_open(crate::window_registry::WindowKey::WorkspaceManager,
                                             cx,
                                             move |cx| {
                                                 open_manager_window(store,
                                                                     messages,
                                                                     settings,
                                                                     Some(mcp_stop),
                                                                     cx)
                                             });
}

/// The manager window itself, reporting its handle.
fn open_manager_window(store: Arc<Mutex<knot_agents::AgentStore>>,
                       messages: Arc<Mutex<knot_messaging::MessageStore>>,
                       settings: knot_core::Settings,
                       mcp_stop: Option<tokio::sync::oneshot::Sender<()>>, cx: &mut App)
                       -> Option<gpui_kit::AnyWindowHandle> {
    let options = manager_window_options(cx);
    match cx.open_window(options, |window, cx| {
                // Every window tracks the OS appearance, so a light/dark flip
                // re-resolves the system palette and repaints.
                observe_system_appearance(window);
                // macOS leaves untitled windows out of the Window menu, which
                // is why only open workspaces were listed
                // there.
                window.set_window_title(&knot_core::l10n::t("workspace.manager"));
                let name_input = cx.new(|cx| {
                                       InputState::new(window, cx)
                        .placeholder(knot_core::l10n::t("workspace.name_placeholder"))
                                   });
                let view = cx.new(|cx| {
                                 let name_subscription =
                                     cx.subscribe(&name_input,
                                                  |_: &mut WorkspaceManager, _, event, cx| {
                                                      if matches!(event, InputEvent::Change) {
                                                          cx.notify();
                                                      }
                                                  });
                                 WorkspaceManager { store,
                                                    messages,
                                                    settings,
                                                    name_input,
                                                    editing_id: None,
                                                    workspace_dialog_id: None,
                                                    show_workspace_dialog: false,
                                                    error: None,
                                                    _name_subscription: name_subscription,
                                                    _mcp_stop: mcp_stop }
                             });
                cx.new(|cx| Root::new(view, window, cx))
            }) {
        Ok(window) => Some(window.into()),
        Err(error) => {
            eprintln!("failed to open workspace manager: {error}");
            None
        }
    }
}

pub(crate) fn run() {
    let mut settings = knot_core::Settings::load().unwrap_or_default();
    if let Err(err) = settings.init_source_folder() {
        eprintln!("failed to initialize source folder: {err}");
    }
    if let Err(err) = settings.install_default_personas() {
        eprintln!("failed to install default personas: {err}");
    }
    quit_on_terminal_signals();
    let store = Arc::new(Mutex::new(build_agent_store(&settings)));
    let notifier = Arc::new(QueuedNotifier::new());
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let awaiting_input = Arc::new(Mutex::new(Vec::new()));
    let activation = Arc::new(Mutex::new(Vec::new()));
    let (mcp_stop, mcp_status) = start_mcp_server(Arc::clone(&store),
                                                  settings.clone(),
                                                  Arc::clone(&notifier),
                                                  Arc::clone(&messages),
                                                  Arc::clone(&awaiting_input),
                                                  Arc::clone(&activation));

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

                               // Before `on_action(quit)`: the guard reads
                               // the store through this global, and a quit
                               // arriving without it would be waved
                               // through unguarded.
                               cx.set_global(quit_guard::QuitGuard::new(Arc::clone(&store)));
                               install_actions_and_keys(&settings, Arc::clone(&store), cx);
                               // The Agents menu starts with nothing
                               // selected, and so disabled; a workspace
                               // window claims it once one is.
                               cx.set_global(AwaitingInput(Arc::clone(&awaiting_input)));
                               cx.set_global(Activation(Arc::clone(&activation)));
                               // The MCP server's state, for the settings
                               // pane's row and the failure notification.
                               cx.set_global(mcp_status);
                               cx.set_global(AgentsMenuState::default());
                               // Every window that can be reopened is
                               // registered here, so a second request for
                               // one raises it rather than making another.
                               crate::window_registry::WindowRegistry::install(cx);
                               register_window_actions(Arc::clone(&store),
                                                       Arc::clone(&messages),
                                                       settings.clone(),
                                                       cx);
                               set_app_menus(&AgentMenuSnapshot::default(), cx);

                               cx.on_system_notification_response(|response, cx| {
                                     // Every notification brings Knot
                                     // forward. An agent's carries its id so
                                     // a click can route to that agent; the
                                     // MCP server's failure is not tied to
                                     // an agent and carries a tag that
                                     // deliberately parses as none - which
                                     // is why activation is no longer gated
                                     // on finding one.
                                     let _agent_to_route_to =
                                         notification_response_agent_id(&response);
                                     cx.activate(true);
                                 });

                               open_workspace_manager(WorkspaceManagerWindow { store:
                                                                                   Arc::clone(&store),
                                                                               messages:
                                                                                   Arc::clone(&messages),
                                                                               settings:
                                                                                   settings.clone(),
                                                                               mcp_stop },
                                                      cx);
                               // macOS launches a non-bundled binary without
                               // making it frontmost, so without this the
                               // window opens behind whatever was already on
                               // screen. `activate` is the app-level
                               // equivalent of ordering the window front.
                               cx.activate(true);
                           });
}
