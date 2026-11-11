//! Menus and shortcuts.

use gpui::{App, KeyBinding, Menu, MenuItem, OsAction, SharedString, SystemMenuType};

macro_rules! actions {
  ($($name:ident),* $(,)?) => {
    $(
      #[derive(Clone, PartialEq, Default, Debug, gpui::Action)]
      #[action(namespace = vista, no_json)]
      pub struct $name;
    )*
  };
}

actions!(
  AboutVista,
  OpenSettings,
  Quit,
  Hide,
  HideOthers,
  ShowAll,
  NewMachine,
  ImportMachine,
  CloseWindow,
  StartMachine,
  ShowMachine,
  PauseMachine,
  ResumeMachine,
  SuspendMachine,
  StopMachine,
  ForceStopMachine,
  SnapshotMachine,
  Minimize,
  Zoom,
  Fullscreen,
  OpenGithub,
  ReportIssue,
  Copy,
  Cut,
  Paste,
  SelectAll,
  Undo,
  Redo,
);

fn menu(name: &'static str, items: Vec<MenuItem>) -> Menu {
  Menu {
    name: SharedString::new_static(name),
    items,
  }
}

pub fn menus() -> Vec<Menu> {
  vec![
    menu(
      "Vista",
      vec![
        MenuItem::action("About Vista", AboutVista),
        MenuItem::separator(),
        MenuItem::action("Settings…", OpenSettings),
        MenuItem::separator(),
        MenuItem::os_submenu("Services", SystemMenuType::Services),
        MenuItem::separator(),
        MenuItem::action("Hide Vista", Hide),
        MenuItem::action("Hide Others", HideOthers),
        MenuItem::action("Show All", ShowAll),
        MenuItem::separator(),
        MenuItem::action("Quit Vista", Quit),
      ],
    ),
    menu(
      "File",
      vec![
        MenuItem::action("New Machine…", NewMachine),
        MenuItem::action("Add Existing Machine…", ImportMachine),
        MenuItem::separator(),
        MenuItem::action("Close Window", CloseWindow),
      ],
    ),
    menu(
      "Edit",
      vec![
        MenuItem::os_action("Undo", Undo, OsAction::Undo),
        MenuItem::os_action("Redo", Redo, OsAction::Redo),
        MenuItem::separator(),
        MenuItem::os_action("Cut", Cut, OsAction::Cut),
        MenuItem::os_action("Copy", Copy, OsAction::Copy),
        MenuItem::os_action("Paste", Paste, OsAction::Paste),
        MenuItem::os_action("Select All", SelectAll, OsAction::SelectAll),
      ],
    ),
    menu(
      "Machine",
      vec![
        MenuItem::action("Start", StartMachine),
        MenuItem::action("Show Window", ShowMachine),
        MenuItem::separator(),
        MenuItem::action("Pause", PauseMachine),
        MenuItem::action("Resume", ResumeMachine),
        MenuItem::action("Suspend", SuspendMachine),
        MenuItem::separator(),
        MenuItem::action("Take Snapshot", SnapshotMachine),
        MenuItem::separator(),
        MenuItem::action("Shut Down", StopMachine),
        MenuItem::action("Force Stop", ForceStopMachine),
      ],
    ),
    menu(
      "Window",
      vec![
        MenuItem::action("Minimize", Minimize),
        MenuItem::action("Zoom", Zoom),
        MenuItem::action("Enter Full Screen", Fullscreen),
      ],
    ),
    menu(
      "Help",
      vec![
        MenuItem::action("Vista on GitHub", OpenGithub),
        MenuItem::action("Report an Issue", ReportIssue),
      ],
    ),
  ]
}

pub fn keys() -> Vec<KeyBinding> {
  vec![
    KeyBinding::new("cmd-,", OpenSettings, None),
    KeyBinding::new("cmd-q", Quit, None),
    KeyBinding::new("cmd-h", Hide, None),
    KeyBinding::new("alt-cmd-h", HideOthers, None),
    KeyBinding::new("cmd-n", NewMachine, None),
    KeyBinding::new("cmd-o", ImportMachine, None),
    KeyBinding::new("cmd-w", CloseWindow, None),
    KeyBinding::new("cmd-r", StartMachine, None),
    KeyBinding::new("cmd-shift-s", SnapshotMachine, None),
    KeyBinding::new("cmd-m", Minimize, None),
    KeyBinding::new("ctrl-cmd-f", Fullscreen, None),
  ]
}

/// App-level handlers: the ones that work with no window open.
pub fn register(cx: &mut App) {
  use crate::state::{library, Page};
  use vista_kit::ipc::Request;

  cx.on_action(|_: &Quit, cx| cx.quit());
  cx.on_action(|_: &Hide, cx| cx.hide());
  cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
  cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
  cx.on_action(|_: &OpenGithub, cx| cx.open_url("https://github.com/wess/vista"));
  cx.on_action(|_: &ReportIssue, cx| cx.open_url("https://github.com/wess/vista/issues/new"));
  cx.on_action(|_: &OpenSettings, cx| {
    library(cx).update(cx, |l, cx| l.select(Page::Settings, cx));
    crate::open_main(cx);
  });
  cx.on_action(|_: &AboutVista, cx| {
    library(cx).update(cx, |l, cx| l.select(Page::Settings, cx));
    crate::open_main(cx);
  });
  cx.on_action(|_: &NewMachine, cx| {
    library(cx).update(cx, |l, cx| l.select(Page::Create, cx));
    crate::open_main(cx);
  });
  cx.on_action(|_: &ImportMachine, cx| {
    let picked = cx.prompt_for_paths(gpui::PathPromptOptions {
      files: false,
      directories: true,
      multiple: false,
      prompt: Some("Add".into()),
    });
    cx.spawn(async move |cx| {
      if let Ok(Ok(Some(paths))) = picked.await {
        if let Some(path) = paths.into_iter().next() {
          let _ = cx.update(|cx| library(cx).update(cx, |l, cx| l.import(path, cx)));
        }
      }
    })
    .detach();
  });

  fn selected(cx: &App) -> Option<String> {
    library(cx).read(cx).selected().map(|m| m.id().to_string())
  }
  fn control(request: Request) -> impl Fn(&mut App) {
    move |cx| {
      if let Some(id) = selected(cx) {
        library(cx).update(cx, |l, cx| l.control(&id, request.clone(), cx));
      }
    }
  }
  cx.on_action(|_: &StartMachine, cx| {
    if let Some(id) = selected(cx) {
      library(cx).update(cx, |l, cx| l.start(&id, true, cx));
    }
  });
  let show = control(Request::Show);
  cx.on_action(move |_: &ShowMachine, cx| show(cx));
  let pause = control(Request::Pause);
  cx.on_action(move |_: &PauseMachine, cx| pause(cx));
  let resume = control(Request::Resume);
  cx.on_action(move |_: &ResumeMachine, cx| resume(cx));
  let suspend = control(Request::Suspend);
  cx.on_action(move |_: &SuspendMachine, cx| suspend(cx));
  let stop = control(Request::Stop);
  cx.on_action(move |_: &StopMachine, cx| stop(cx));
  let kill = control(Request::Kill);
  cx.on_action(move |_: &ForceStopMachine, cx| kill(cx));
  cx.on_action(|_: &SnapshotMachine, cx| {
    if let Some(id) = selected(cx) {
      library(cx).update(cx, |l, cx| l.snapshot(&id, String::new(), cx));
    }
  });
}
