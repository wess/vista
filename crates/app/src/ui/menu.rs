use gpui::{App, AppContext, Entity, Window};
use guise::prelude::*;
use vista_kit::{ipc::Request, machine::State};

use crate::state::{Library, Machine, Notice};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Command {
  Start,
  Background,
  Show,
  Hide,
  Pause,
  Resume,
  Suspend,
  Stop,
  ForceStop,
  Snapshot,
  Duplicate,
  Reveal,
  Delete,
}

fn available(command: Command, state: State, process: bool, files: bool, visible: bool) -> bool {
  use Command::*;
  match command {
    Start | Background => files && !process,
    Show => process && !visible,
    Hide => process && visible,
    Pause | Suspend => process && state == State::Running,
    Resume => process && state == State::Paused,
    Stop => process && matches!(state, State::Running | State::Paused),
    ForceStop => process,
    Snapshot => {
      files
        && matches!(
          (state, process),
          (State::Stopped, false) | (State::Running, true)
        )
    }
    Duplicate => files && !process,
    Reveal => files,
    Delete => !process,
  }
}

fn allowed(command: Command, machine: &Machine) -> bool {
  available(
    command,
    machine.state(),
    machine.live.is_some(),
    machine.bundle.is_some(),
    machine.live.as_ref().is_some_and(|live| live.visible),
  )
}

fn run(
  command: Command,
  id: &str,
  library: &Entity<Library>,
  overlays: &Entity<OverlayHost>,
  window: &mut Window,
  cx: &mut App,
) {
  let Some(machine) = library.read(cx).machine(id).cloned() else {
    return;
  };
  if !allowed(command, &machine) {
    library.update(cx, |_, cx| {
      cx.emit(Notice::Info(
        "The machine’s state changed. Open its menu again for available actions.".into(),
      ))
    });
    return;
  }
  use Command::*;
  match command {
    Reveal => {
      if let Some(bundle) = &machine.bundle {
        cx.reveal_path(&bundle.path);
      }
    }
    Delete | ForceStop => {
      let id = id.to_string();
      let library = library.clone();
      let (title, message, action) = if command == ForceStop {
        (
          format!("Force stop {}?", machine.name()),
          "Unsaved work in the machine may be lost.",
          "Force Stop",
        )
      } else if machine.bundle.is_some() {
        (
          format!("Delete {}?", machine.name()),
          "Its disks and snapshots move to the Trash.",
          "Delete",
        )
      } else {
        (
          format!("Remove {} from the library?", machine.name()),
          "This removes the missing machine’s library entry.",
          "Remove",
        )
      };
      super::detail::ask(overlays, &title, message, action, window, cx, move |cx| {
        library.update(cx, |library, cx| {
          if command == Delete {
            library.delete(&id, cx);
          } else if library
            .machine(&id)
            .is_some_and(|machine| machine.live.is_some())
          {
            library.control(&id, Request::Kill, cx);
          }
        });
      });
    }
    _ => library.update(cx, |library, cx| match command {
      Start => library.start(id, true, cx),
      Background => library.start(id, false, cx),
      Snapshot => library.snapshot(id, String::new(), cx),
      Duplicate => library.duplicate(id, cx),
      _ => {
        let request = match command {
          Show => Request::Show,
          Hide => Request::Hide,
          Pause => Request::Pause,
          Resume => Request::Resume,
          Suspend => Request::Suspend,
          Stop => Request::Stop,
          _ => return,
        };
        library.control(id, request, cx);
      }
    }),
  }
}

pub fn machine(
  machine: &Machine,
  library: &Entity<Library>,
  overlays: &Entity<OverlayHost>,
  cx: &mut App,
) -> Entity<ContextMenu> {
  let machine = machine.clone();
  let library = library.clone();
  let overlays = overlays.clone();
  cx.new(move |cx| {
    use Command::*;
    let start = if machine.state() == State::Suspended {
      "Resume"
    } else {
      "Start"
    };
    let groups = [
      vec![
        (Start, start),
        (
          Background,
          if machine.state() == State::Suspended {
            "Resume in Background"
          } else {
            "Start in Background"
          },
        ),
        (Show, "Show Window"),
        (Hide, "Hide Window"),
        (Pause, "Pause"),
        (Resume, "Resume"),
        (Suspend, "Suspend"),
        (Stop, "Shut Down"),
        (ForceStop, "Force Stop…"),
      ],
      vec![
        (Snapshot, "Take Snapshot"),
        (Duplicate, "Clone"),
        (Reveal, "Show in Finder"),
      ],
      vec![(
        Delete,
        if machine.bundle.is_some() {
          "Delete…"
        } else {
          "Remove from Library…"
        },
      )],
    ];
    let mut menu = ContextMenu::new(cx).width(236.);
    let mut populated = false;
    for group in groups {
      let entries: Vec<_> = group
        .into_iter()
        .filter(|(command, _)| allowed(*command, &machine))
        .collect();
      if entries.is_empty() {
        continue;
      }
      if populated {
        menu = menu.divider();
      }
      populated = true;
      for (command, label) in entries {
        let library = library.clone();
        let overlays = overlays.clone();
        let id = machine.id().to_string();
        let handler = move |window: &mut Window, cx: &mut App| {
          run(command, &id, &library, &overlays, window, cx)
        };
        menu = if matches!(command, Delete | ForceStop) {
          menu.danger_item(label, handler)
        } else {
          menu.item(label, handler)
        };
      }
    }
    menu
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn lifecycle_actions_follow_process_and_state() {
    assert!(available(
      Command::Start,
      State::Suspended,
      false,
      true,
      false
    ));
    assert!(!available(Command::Start, State::Running, true, true, true));
    assert!(available(Command::Pause, State::Running, true, true, true));
    assert!(!available(Command::Pause, State::Paused, true, true, true));
    assert!(available(Command::Resume, State::Paused, true, true, true));
    assert!(!available(
      Command::Snapshot,
      State::Installing,
      true,
      true,
      true
    ));
    assert!(!available(
      Command::Duplicate,
      State::Saving,
      true,
      true,
      false
    ));
    assert!(!available(Command::Delete, State::Error, true, true, true));
    assert!(available(
      Command::ForceStop,
      State::Error,
      true,
      true,
      true
    ));
  }

  #[test]
  fn missing_machines_can_only_be_removed() {
    for command in [
      Command::Start,
      Command::Background,
      Command::Snapshot,
      Command::Duplicate,
      Command::Reveal,
    ] {
      assert!(!available(command, State::Stopped, false, false, false));
    }
    assert!(available(
      Command::Delete,
      State::Stopped,
      false,
      false,
      false
    ));
  }

  #[test]
  fn visibility_actions_are_mutually_exclusive() {
    for visible in [false, true] {
      assert_eq!(
        available(Command::Show, State::Running, true, true, visible),
        !visible
      );
      assert_eq!(
        available(Command::Hide, State::Running, true, true, visible),
        visible
      );
    }
  }
}
