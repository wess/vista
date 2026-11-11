# Snapshots, suspend and clones

Three ways to keep a machine's state, which save different things.

| | Saves | Machine afterwards | Lives in |
| --- | --- | --- | --- |
| **Pause** | Nothing: the guest's CPUs stop | Still running, frozen in memory | memory |
| **Suspend** | Memory | Off; the next start resumes exactly where it was | `suspended.vzvmsave` |
| **Snapshot** | Disks and firmware state, plus memory if taken while running | Unchanged | `snapshots/<id>/` |
| **Clone** | A whole new machine | Unchanged | a new `.vista` folder |

## Snapshots

On the **Snapshots** tab, name a snapshot (or leave it blank for
"Snapshot N") and choose **Take Snapshot**. In a machine's window it's
**Machine → Take Snapshot**; in Vista's menu bar it's **Machine → Take
Snapshot** (⇧⌘S).

**Snapshots are copy-on-write.** On an APFS volume (every Mac's internal
disk), Vista clones the machine's files with `clonefile`: a snapshot of a
64 GB disk is made in milliseconds and takes no space at first. It costs
space only as the machine writes afterwards, roughly the amount it changes.
On a volume without APFS, snapshots are full copies.

**Snapshots of a running machine include its memory.** Vista pauses the
guest for a moment, writes its memory into the snapshot, clones the disks and
resumes. Such snapshots are marked **With memory**. Some hardware
configurations can't have their memory saved; Vista says so, and you can
snapshot them while they're off.

### Reverting

**Revert** puts the machine back exactly as the snapshot found it, discarding
everything since. A running machine is stopped first (Vista asks). Reverting
to a snapshot with memory leaves the machine suspended at that moment:
**Start** resumes into it, apps and all.

The machine's configuration is reverted too, since the memory only makes sense
with the hardware it was saved with.

### Deleting

Deleting a snapshot frees the space only it was using. The machine isn't
affected. Agents can also rename snapshots.

## Suspend

**Suspend** (header button, the machine window's Machine menu, or closing the
machine's window) writes the guest's memory to the machine's folder and quits
its process. **Start** — now labelled **Resume** — restores it in seconds.

The saved memory is used once. If you change the machine's hardware while it's
suspended, the saved memory no longer fits and the next start is a fresh boot,
as if the power had been cut; Vista logs that in `machine.log`.

## Clones

**Clone** (Overview tab) makes a complete copy of a stopped machine next to
it, named "*name* copy". Like snapshots it's a copy-on-write clone: instant,
and free until either machine changes.

The clone is a different machine, not a twin: it gets a new id, a new MAC
address (so a different IP address) and a new platform identity, which the
guest sees as different hardware. Snapshots aren't copied. A suspended
machine has to be resumed and shut down before it can be cloned.
