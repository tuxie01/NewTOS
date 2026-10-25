//! AES process descriptors (`AESPD`) — per-application AES state.
//!
//! Spec: [`docs/api/aes/application.md`] § "AES process identity and pid",
//! contract (a). Every AES client has a descriptor **by construction**: the
//! table has one entry per GEMDOS process slot plus one for the kernel
//! context ([`NUM_PDS`] = `MAX_PROCESSES + 1`), and a client's **identity** is
//! its index — the GEMDOS slot for an EL0 process, [`KERNEL_CONTEXT`] for the boot
//! context (`appl::current_identity`). The descriptor holds the client's
//! `p_flags` (`AP_OPEN`/`AP_MESAG`/…), its 8-char name (for `appl_find`), and
//! a per-application **FIFO message queue** (the standard GEM message pipe
//! used by `evnt_mesag`/`evnt_multi`/`appl_read`).
//!
//! This is EmuTOS's shape: its `AESPD`s are fixed entries of
//! `D.g_int[]`/`D.g_acc[]` reached by index (`pd_index`, `aes/gempd.c:32-35`),
//! initialised once at boot (`geminit.c:851-877`) and never allocated or freed
//! afterwards — which is why `ap_init` (`aes/gemaplib.c:54-63`) has no failure
//! path. NewTOS's earlier eight-entry table, searched by pid, could fill up
//! (measured: `appl_init` = −1 from the ninth nested `Pexec`, A62); indexing
//! by the slot leaves nothing to fill. The **reported** pid a program sees
//! (`ap_id`, `appl_find`, `WF_OWNER`, `msg[1]`) is computed from the identity
//! by `appl::reported_pid` and is not stored here.
//!
//! The queue mirrors EmuTOS's `p_queue`/`p_qindex`/`p_qaddr` byte queue
//! (`aes/struct.h:150-152`, `QUEUE_SIZE = 128` bytes = 8 standard 16-byte
//! messages, `aes/struct.h:56`) and the enqueue/dequeue + WM_REDRAW
//! rectangle-coalescing rules of `doq`/`aqueue` (`aes/gemqueue.c:31-125`).
//! A single pending slot is just the 1-message case of this queue; when the
//! desktop (one AES app owning many windows) posts several `WM_REDRAW`s
//! before the event loop drains them, they no longer overwrite each other.
//!
//! Backed by a static slot table accessed only from the single-CPU,
//! IRQ-off GEM-trap path (the same discipline as the VDI/`Vwk` table).

use core::cell::UnsafeCell;
use tos_types::aes::{WM_ARROWED, WM_HSLID, WM_REDRAW, WM_VSLID};

/// `p_flags` bit: the application is between `appl_init` and `appl_exit`.
pub const AP_OPEN: u16 = 0x0001;
/// `p_flags` bit: the application has waited for a message.
pub const AP_MESAG: u16 = 0x0002;
/// `p_flags` bit: the application has seen an `AC_CLOSE` message
/// (`aes/struct.h:117`). Set by the Event Manager when a `MU_MESAG` delivery
/// hands out an `AC_CLOSE` (`gemsuper.c:139-140,160-161`) — never by the
/// accessory itself — and read by `wait_for_accs(AP_ACCLOSE)`
/// (`geminit.c:599-628`, called from `ap_exit`, `gemaplib.c:255`). EmuTOS
/// never clears it: once a descriptor has seen one `AC_CLOSE`, every later
/// `wait_for_accs(AP_ACCLOSE)` passes for it immediately.
pub const AP_ACCLOSE: u16 = 0x0004;

/// Number of AES process descriptors: one per GEMDOS process slot
/// (`bdos::process::MAX_PROCESSES`) plus one for the kernel context.
/// The table is full-size from boot; nothing is allocated into it.
pub const NUM_PDS: usize = crate::bdos::process::MAX_PROCESSES + 1;

/// Identity of the kernel context — the one AES client that is not a GEMDOS
/// process: kernel code calling the AES with `RUN` null (the shell loop's
/// `sh_draw`/`fm_error`, the boot path, the boot-time selftests). Its own
/// index keeps it apart from GEMDOS slot 0. EmuTOS's counterpart is its
/// initial AES process, `pd_index(0)` = `D.g_int[0].a_pd` with `p_pid` 0
/// (`aes/gempd.c:32-35`, `aes/geminit.c:850-876`), which runs the shell loop;
/// EmuTOS gives it no name of its own. Until DESK43 U10 the built-in desktop
/// was this client (hence its former name `DESKTOP`); since U9/U10 the desktop
/// is the EL0 program `NEWDESK` with a slot of its own.
pub const KERNEL_CONTEXT: usize = crate::bdos::process::MAX_PROCESSES;

/// Length of the standard GEM message (8 WORDs = 16 bytes) — the normal case
/// for every internal AES poster (`WM_REDRAW`, menu, keyboard, …).
pub const MSG_WORDS: usize = 8;

/// EmuTOS per-process queue size in bytes (`aes/struct.h:56`,
/// `p_queue[QUEUE_SIZE]` at `:152`). Also the largest a single message may be:
/// EmuTOS `doq` memcpys up to `qpb_cnt` bytes into a queue of this size
/// (`aes/gemqueue.c:36,97`).
pub const QUEUE_SIZE: usize = 128;
/// Largest single message in WORDs: `QUEUE_SIZE / 2 = 64`. `appl_write` clamps
/// `AP_LENGTH` to this — the per-message FIFO limit (`aes/gemqueue.c` never
/// enqueues more than `QUEUE_SIZE` bytes). The standard 16-byte message uses
/// only the first [`MSG_WORDS`] of it.
pub const MAX_MSG_WORDS: usize = QUEUE_SIZE / 2;
/// Maximum number of messages the FIFO holds at once, kept at the original
/// `QUEUE_SIZE / 16 = 8` (enough for one `WM_REDRAW` per open window before the
/// event loop drains them). Each slot can hold a full [`MAX_MSG_WORDS`]-WORD
/// message; what the queue *accepts* is bounded by EmuTOS's shared 128-byte
/// budget as well ([`MsgQueue::fits`], A116), so the slots only over-provision
/// storage, not capacity.
pub const QUEUE_MSGS: usize = QUEUE_SIZE / (MSG_WORDS * 2);

// GEM message types that `doq` coalesces when enqueued — `WM_REDRAW`,
// `WM_ARROWED`, `WM_HSLID`, `WM_VSLID` (`tos_types::aes`), used by the
// queue-union rule in `aes/gemqueue.c:45-67`.

/// Per-application GEM message FIFO — the reimplementation of EmuTOS's
/// `p_queue`/`p_qindex` byte queue. Holds up to [`QUEUE_MSGS`] messages, each
/// of up to [`MAX_MSG_WORDS`] WORDs (`QUEUE_SIZE` bytes); the standard case is
/// the 16-byte (8-WORD) GEM message. `count` is the number of messages
/// currently queued (the analog of `p_qindex`, kept in whole messages here
/// because the slot model delivers a message atomically rather than as a byte
/// stream). Unlike EmuTOS the FIFO cannot split a message across reads: a
/// message is enqueued and dequeued whole (see [`Self::pop_into`]).
#[derive(Clone, Copy)]
pub struct MsgQueue {
    /// Each queued message, held in up to [`MAX_MSG_WORDS`] WORDs
    /// (`QUEUE_SIZE` bytes); only the first `lens[i]` WORDs are valid.
    msgs: [[i16; MAX_MSG_WORDS]; QUEUE_MSGS],
    /// Valid WORD length of each queued message (`msgs[i][..lens[i]]`). The
    /// standard GEM message is [`MSG_WORDS`]; `appl_write` may queue up to
    /// [`MAX_MSG_WORDS`].
    lens: [usize; QUEUE_MSGS],
    count: usize,
}

impl MsgQueue {
    const EMPTY: MsgQueue = MsgQueue {
        msgs: [[0; MAX_MSG_WORDS]; QUEUE_MSGS],
        lens: [0; QUEUE_MSGS],
        count: 0,
    };

    /// Number of messages currently queued (the analog of `p_qindex`; a read
    /// is ready when this is `> 0`, EmuTOS `aes/gemqueue.c:99`).
    pub fn pending(&self) -> usize {
        self.count
    }

    /// Bytes currently queued — EmuTOS's `p_qindex` proper, the byte fill of
    /// the 128-byte `p_queue` (`aes/struct.h:150-152`).
    pub fn bytes(&self) -> usize {
        self.lens[..self.count].iter().map(|&w| 2 * w).sum()
    }

    /// Would a `words`-WORD message fit — `aqueue`'s write readiness
    /// `qpb_cnt <= QUEUE_SIZE - p_qindex` (`aes/gemqueue.c:96-97`), tested
    /// **before** any coalescing (`doq` runs only once `qready` holds,
    /// `:104-106`). The slot model adds its own bound: at most
    /// [`QUEUE_MSGS`] messages, reached before the byte bound only by
    /// messages shorter than the standard 16 bytes.
    pub fn fits(&self, words: usize) -> bool {
        let need = 2 * words.min(MAX_MSG_WORDS);
        self.count < QUEUE_MSGS && need <= QUEUE_SIZE - self.bytes()
    }

    /// Peek the standard 16-byte front (oldest) message without removing it.
    /// Used by the P5 test hook; the real `evnt_*`/`appl_read` paths
    /// [`pop`](Self::pop)/[`pop_into`](Self::pop_into).
    pub fn front(&self) -> Option<[i16; MSG_WORDS]> {
        if self.count == 0 {
            return None;
        }
        let mut out = [0i16; MSG_WORDS];
        let n = self.lens[0].min(MSG_WORDS);
        out[..n].copy_from_slice(&self.msgs[0][..n]);
        Some(out)
    }

    /// Enqueue the standard 16-byte GEM message `msg` — [`Self::enqueue`] at
    /// the standard [`MSG_WORDS`] length. Used by every internal poster
    /// (`WM_REDRAW`, menu, keyboard). Returns `false` when the queue is full.
    pub fn push(&mut self, msg: [i16; MSG_WORDS]) -> bool {
        self.enqueue(&msg, MSG_WORDS)
    }

    /// Enqueue a variable-length message: the first `words` WORDs of `src` —
    /// the `appl_write`/`ap_rdwr(MU_SDMSG)` → `doq` path (EmuTOS
    /// `aes/gemqueue.c:31-73`, which memcpys `qpb_cnt` bytes). `words` is
    /// clamped to [`MAX_MSG_WORDS`] (the per-message FIFO limit) and to
    /// `src.len()`. Returns `false` when the queue is full.
    pub fn push_words(&mut self, src: &[i16], words: usize) -> bool {
        self.enqueue(src, words.min(src.len()))
    }

    /// Core enqueue for [`Self::push`] and [`Self::push_words`] — the enqueue
    /// half of EmuTOS `doq` (`aes/gemqueue.c:37-73`). Appends the first `words`
    /// WORDs of `src` at the tail, except that a standard-length
    /// `WM_REDRAW`/`WM_ARROWED`/`WM_HSLID`/`WM_VSLID` (`:45`) is first coalesced
    /// into an already-queued standard message:
    ///
    /// * a `WM_REDRAW` for the **same window handle** (`src[3]`): union the new
    ///   rectangle (`src[4..8]`) into that entry via `rc_union`
    ///   (`aes/gemqueue.c:52-56`, `util/rectfunc.c:84-97`) and drop the new
    ///   message — the fix for redraw flooding / lost content redraws;
    /// * otherwise an existing `WM_ARROWED`: overwrite it (`aes/gemqueue.c:63-67`).
    ///
    /// Only standard ([`MSG_WORDS`]-long) messages take part in coalescing — the
    /// coalesced message types are always 16 bytes.
    ///
    /// Returns `false` when the message does not fit ([`Self::fits`]: the
    /// 128-byte budget of `aqueue`'s `qpb_cnt <= QUEUE_SIZE - p_qindex`,
    /// `aes/gemqueue.c:96-97`, and the slot bound). A coalescing message needs
    /// no room and is always taken — here, for the kernel-side posters, the
    /// coalescing comes first; `appl_write` tests [`Self::fits`] itself before
    /// calling this, in EmuTOS's order (A116).
    ///
    /// **Kernel-side posters do not block on a full queue — a named
    /// deviation.** In EmuTOS every internal poster goes through `ap_sendmsg`
    /// → `ap_rdwr(MU_SDMSG)` and blocks like `appl_write` (`aqueue`,
    /// `gemqueue.c:120-124`). NewTOS's posters (`post_msg`: `WM_REDRAW`
    /// fan-out, the inline Control Manager's `WM_*`/`MN_SELECTED`, `AC_CLOSE`)
    /// run inline inside whatever AES call caused them and cannot park and
    /// re-execute that call, so a post into a full queue is dropped
    /// (`docs/COMPATIBILITY.md` "`appl_write` on a full queue", A75/A116).
    fn enqueue(&mut self, src: &[i16], words: usize) -> bool {
        let words = words.min(MAX_MSG_WORDS);
        if words == MSG_WORDS {
            let t = src[0];
            if t == WM_REDRAW || t == WM_ARROWED || t == WM_HSLID || t == WM_VSLID {
                for i in 0..self.count {
                    if self.lens[i] != MSG_WORDS {
                        continue; // only standard messages coalesce
                    }
                    let om = &mut self.msgs[i];
                    if om[0] == WM_REDRAW && src[3] == om[3] {
                        // Union the new rectangle into the existing entry.
                        rc_union_into(src, om);
                        return true;
                    } else if om[0] == WM_ARROWED {
                        // Replace the pending arrow with the newer message.
                        om[..MSG_WORDS].copy_from_slice(&src[..MSG_WORDS]);
                        return true;
                    }
                }
            }
        }
        if !self.fits(words) {
            return false;
        }
        let slot = &mut self.msgs[self.count];
        slot[..words].copy_from_slice(&src[..words]);
        for w in &mut slot[words..] {
            *w = 0;
        }
        self.lens[self.count] = words;
        self.count += 1;
        true
    }

    /// Dequeue the standard 16-byte front (oldest) message — the dequeue half
    /// of EmuTOS `doq` (`aes/gemqueue.c:76-81`). Returns the first [`MSG_WORDS`]
    /// WORDs of `p_queue[0]` and shifts the residual messages down; `None` when
    /// the queue is empty. Used by the internal `evnt_mesag`/`evnt_multi` paths,
    /// which read exactly 16 bytes (EmuTOS `ev_mesag` → `ap_rdwr(…,16,…)`).
    pub fn pop(&mut self) -> Option<[i16; MSG_WORDS]> {
        if self.count == 0 {
            return None;
        }
        let mut out = [0i16; MSG_WORDS];
        let n = self.lens[0].min(MSG_WORDS);
        out[..n].copy_from_slice(&self.msgs[0][..n]);
        self.remove_front();
        Some(out)
    }

    /// Dequeue the front message into `dst`, copying at most `dst.len()` WORDs
    /// (the `appl_read`/`ap_rdwr(MU_MESAG,…,length,…)` caller's requested
    /// length); returns the number of WORDs delivered, or `None` when the queue
    /// is empty. The whole message is removed even if `dst` is shorter than it —
    /// the slot FIFO delivers a message atomically, without EmuTOS's byte-stream
    /// partial reads (see the type-level note).
    pub fn pop_into(&mut self, dst: &mut [i16]) -> Option<usize> {
        if self.count == 0 {
            return None;
        }
        let n = self.lens[0].min(dst.len());
        dst[..n].copy_from_slice(&self.msgs[0][..n]);
        self.remove_front();
        Some(n)
    }

    /// Remove the front message, shifting the residual messages down (EmuTOS
    /// `doq` dequeue, `aes/gemqueue.c:76-81`).
    fn remove_front(&mut self) {
        for i in 1..self.count {
            self.msgs[i - 1] = self.msgs[i];
            self.lens[i - 1] = self.lens[i];
        }
        self.count -= 1;
    }

    /// Discard every queued message (explicit queue reset and test hook).
    pub fn clear(&mut self) {
        self.count = 0;
    }
}

/// Union the rectangle in `p1[4..8]` into `p2[4..8]`, storing the result in
/// `p2` — EmuTOS `rc_union(p1, p2)` (`util/rectfunc.c:84-97`), used by `doq`
/// to coalesce two same-window `WM_REDRAW` rectangles (`aes/gemqueue.c:54`).
/// Both slices are the coalesced standard messages, so index `4..8` is valid.
fn rc_union_into(p1: &[i16], p2: &mut [i16]) {
    let tw = (p1[4] + p1[6]).max(p2[4] + p2[6]);
    let th = (p1[5] + p1[7]).max(p2[5] + p2[7]);
    let tx = p1[4].min(p2[4]);
    let ty = p1[5].min(p2[5]);
    p2[4] = tx;
    p2[5] = ty;
    p2[6] = tw - tx;
    p2[7] = th - ty;
}

/// One AES application descriptor — the entry for one identity (its index in
/// the table). The pid is not stored: the identity *is* the index, and the
/// reported value is computed from it (`appl::reported_pid`).
#[derive(Clone, Copy)]
pub struct Aespd {
    pub flags: u16,
    pub name: [u8; 8],
    /// The application's pending GEM message FIFO (`p_queue`/`p_qindex`).
    pub queue: MsgQueue,
}

impl Aespd {
    /// The boot state of every entry — EmuTOS's init loop
    /// (`geminit.c:866-875`): blank `p_name`, empty queue, no flags.
    const EMPTY: Aespd = Aespd {
        flags: 0,
        name: [b' '; 8],
        queue: MsgQueue::EMPTY,
    };
}

struct Table(UnsafeCell<[Aespd; NUM_PDS]>);
// SAFETY: accessed only from the single-CPU, IRQ-off GEM-trap path.
unsafe impl Sync for Table {}
static TABLE: Table = Table(UnsafeCell::new([Aespd::EMPTY; NUM_PDS]));

/// Borrow the descriptor table.
///
/// # Safety
/// Single trap path (no re-entrancy).
#[allow(clippy::mut_from_ref)]
unsafe fn table() -> &'static mut [Aespd; NUM_PDS] {
    // SAFETY: see `Table`'s Sync impl.
    unsafe { &mut *TABLE.0.get() }
}

/// Put every descriptor back to its power-up state — the process-table half
/// of an AES restart (A106, [`super::reinit`]).
///
/// EmuTOS's equivalent is two steps of `gem_main`: `aes_restart` re-zeroes
/// the whole `theglo` area the `AESPD`s and their message queues live in
/// (`gemstart.S:101-110`, `aes/gemlib.h:118`), and the init loop then walks
/// every descriptor blanking `p_name`, `p_qindex`, `p_stat` and `p_msg`
/// (`geminit.c:846-880`). [`Aespd::EMPTY`] is exactly that state: no flags,
/// a blank name, an empty queue.
///
/// Every EL0 process is gone by the time this runs — `sh_main` returns only
/// after the program it started has terminated — so no live client loses a
/// descriptor here; what goes is the residue of the session that ended.
///
/// # Safety
/// Single trap / restart path (no AES call in flight).
pub unsafe fn reinit() {
    // SAFETY: caller upholds the single-path invariant.
    let t = unsafe { table() };
    for slot in t.iter_mut() {
        *slot = Aespd::EMPTY;
    }
    // …and no write is parked on any of them (`p_qnq` is part of the zeroed
    // `AESPD`, `aes/struct.h:139`).
    // SAFETY: as above.
    let (w, next) = unsafe { writers() };
    *w = [ParkedWrite::IDLE; NUM_PDS];
    *next = 0;
}

/// Address of the AES process-descriptor table — NewTOS's nearest equivalent
/// of EmuTOS's AES global data area `THEGLO D` (`aes/gemlib.h:116-144`, whose
/// `g_int[]` holds the `AESPD` process descriptors; a single unified `D` struct
/// with no NewTOS counterpart — our AES state is split across statics, of which
/// this descriptor table is the process-management core). `appl_init` reports it
/// in `ap_3resv` (EmuTOS `pglobal->ap_3resv = (LONG)&D`, `gemsuper.c:98`).
pub fn table_addr() -> u64 {
    TABLE.0.get() as u64
}

/// Open the descriptor of identity `idx` — EmuTOS `ap_init`
/// (`aes/gemaplib.c:54-63`): sets `AP_OPEN` and nothing else, preserving the
/// queue, name and other flags (A25). Cannot fail: the descriptor exists from
/// boot, as EmuTOS's does, so there is no "table full" to report and
/// `appl_init` never returns −1 for it (A62). `idx` is an identity
/// (`appl::current_identity`: a GEMDOS slot `< MAX_PROCESSES` or [`KERNEL_CONTEXT`]),
/// always inside the table.
///
/// The application's loaded resources are not part of this descriptor
/// (EmuTOS's `AESPD` has no resource field either, `aes/struct.h:119-155`);
/// they are keyed by the same identity in `super::resource` and dropped at
/// the process's end (`resource::on_process_exit`), so a reused descriptor
/// never carries a previous tenant's resource (A39 R6).
///
/// # Safety
/// Single trap path.
pub unsafe fn open(idx: usize) -> &'static mut Aespd {
    // SAFETY: caller upholds the single-trap-path invariant.
    let t = unsafe { table() };
    let slot = &mut t[idx];
    slot.flags |= AP_OPEN;
    slot
}

#[cfg(test)]
mod init_tests {
    use super::*;

    #[test]
    fn every_identity_has_a_descriptor_and_reopening_preserves_flags_and_queue() {
        // This unit-test binary has no other descriptor-table users.
        unsafe {
            for idx in 0..NUM_PDS {
                let app = open(idx);
                assert_eq!(app.flags & AP_OPEN, AP_OPEN, "identity {idx}");
            }
            let message = [0x4711, 1, 0, 2, 3, 4, 5, 6];
            {
                let app = by_index(3).unwrap();
                app.flags |= AP_MESAG;
                app.name = *b"INITTEST";
                assert!(app.queue.push(message));
            }
            assert!(by_index(NUM_PDS).is_none(), "no descriptor beyond the table");
            {
                let app = open(3);
                assert_eq!(app.flags, AP_OPEN | AP_MESAG, "ap_init only ORs AP_OPEN");
                assert_eq!(app.name, *b"INITTEST");
                let mut received = [0; MSG_WORDS];
                assert_eq!(app.queue.pop_into(&mut received), Some(MSG_WORDS));
                assert_eq!(received, message);
            }
            // A slot's release returns its entry to the boot state.
            release(3);
            let app = by_index(3).unwrap();
            assert_eq!(app.flags, 0);
            assert_eq!(app.queue.pending(), 0);
            for idx in 0..NUM_PDS {
                close(idx);
            }
        }
    }
}

/// Post a 16-byte GEM message into identity `idx`'s queue — the routing point
/// every AES message post goes through (the `ap_rdwr(MU_SDMSG)` → `aqueue` →
/// `doq` enqueue path, `aes/gemqueue.c:85-125,31-74`). Returns `false` if `idx`
/// is outside the table or the queue is full (see [`MsgQueue::push`]).
///
/// # Safety
/// Single trap path.
pub unsafe fn post_msg(idx: usize, msg: [i16; MSG_WORDS]) -> bool {
    // SAFETY: caller upholds the single-trap-path invariant.
    let posted = match unsafe { by_index(idx) } {
        Some(p) => p.queue.push(msg),
        None => false,
    };
    // AES trap tracer: record the enqueue (type, sender, target as the value
    // a program would see, and whether the FIFO took it). No-op without the
    // `aes-trace` feature.
    super::trace::msg_post(super::appl::reported_pid(idx), &msg, MSG_WORDS, posted);
    if posted {
        wake_mesg_waiter(idx);
    }
    posted
}

/// Post a variable-length message into identity `idx`'s queue — the
/// `appl_write` / `ap_rdwr(MU_SDMSG)` path with a caller-supplied byte length
/// (EmuTOS `doq` memcpys `qpb_cnt` bytes, `aes/gemqueue.c:31-73`). Enqueues the
/// first `words` WORDs of `src` with the standard redraw/arrow coalescing
/// ([`MsgQueue::push_words`]) and, on success, wakes an AES client blocked on
/// `idx`'s FIFO through the **same** centralized wake as [`post_msg`] — so
/// `appl_write` no longer bypasses it (design §5.2). Returns `false` if `idx`
/// is outside the table or the queue is full.
///
/// # Safety
/// Single trap path.
pub unsafe fn post_words(idx: usize, src: &[i16], words: usize) -> bool {
    // SAFETY: caller upholds the single-trap-path invariant.
    let posted = match unsafe { by_index(idx) } {
        Some(p) => p.queue.push_words(src, words),
        None => false,
    };
    // AES trap tracer, as in `post_msg` (this is the `appl_write` half).
    super::trace::msg_post(super::appl::reported_pid(idx), src, words, posted);
    if posted {
        wake_mesg_waiter(idx);
    }
    posted
}

/// Wake any AES client blocked in `evnt_mesag`/`evnt_multi`/`appl_read` on
/// identity `idx`'s message FIFO (WAITING → READY, sleep/wake substrate §5.2). The
/// single wake point for every message poster ([`post_msg`]/[`post_words`]),
/// matching EmuTOS's `aqueue`→`azombie`→`signal`
/// (`emutos-cooperative-model.md` §3.3/§4.3). Not load-bearing in step 3a
/// (single context: the waiter's own idle re-check drains the queue), but
/// wired now so step 4's real peer switch needs no `event.rs`/`appl_write`
/// change.
fn wake_mesg_waiter(idx: usize) {
    crate::bdos::process::sched::wake(crate::bdos::process::sched::wait_mesg(idx));
}

// ─── parked writers: EmuTOS `p_qnq` (A116) ──────────────────────────────────
//
// `aqueue` parks a write that does not fit on the target's `p_qnq` list
// (`aes/gemqueue.c:96-101,120-124`) and the writer sleeps in `mwait`
// (`aes/gemasync.c:117-127`). A later **successful read of that same queue**
// services one entry of the list — unconditionally, the head — with `doq` +
// `azombie(e, 1)` (`:108-118`), so the parked `appl_write` returns 1.
// `evinsert` links a new `EVB` in at the **head** of the list
// (`aes/gemasync.c:81-93`) and the service takes the head, so of several
// parked writers the most recently parked is served first.
//
// The list is kept here as one entry per **writer** identity (an identity has
// at most one call in flight), ordered by a park sequence number. The writer's
// message is copied in at park time: NewTOS's writer runs in its own address
// space, and the read that services it runs in the reader's. EmuTOS copies
// from the writer's buffer at service time; the writer is blocked in between,
// so only another process scribbling over that buffer could tell the two apart.

/// State of one identity's `appl_write` with respect to `p_qnq`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum WriteState {
    /// No write of this identity is parked.
    Idle,
    /// Parked on `target`'s full queue (`evinsert(e, &p->p_qnq)`).
    Parked,
    /// Serviced by a read (`azombie(e, 1)`); the re-executed call returns 1.
    Done,
}

/// One identity's parked `appl_write` — the `EVB` + `QPB` EmuTOS links onto
/// `p_qnq`.
#[derive(Clone, Copy)]
struct ParkedWrite {
    state: WriteState,
    /// Identity whose queue the write waits for.
    target: usize,
    /// Park order; the largest parked `seq` on a target is the list head.
    seq: u64,
    /// Valid WORDs of `msg` (`qpb_cnt / 2`).
    words: usize,
    msg: [i16; MAX_MSG_WORDS],
}

impl ParkedWrite {
    const IDLE: ParkedWrite = ParkedWrite {
        state: WriteState::Idle,
        target: 0,
        seq: 0,
        words: 0,
        msg: [0; MAX_MSG_WORDS],
    };
}

/// The parked writes, one per writer identity, and the next park sequence.
struct Writers(UnsafeCell<([ParkedWrite; NUM_PDS], u64)>);
// SAFETY: accessed only from the single-CPU, IRQ-off GEM-trap path.
unsafe impl Sync for Writers {}
static WRITERS: Writers = Writers(UnsafeCell::new(([ParkedWrite::IDLE; NUM_PDS], 0)));

/// Borrow the parked-write table.
///
/// # Safety
/// Single trap path (no re-entrancy).
#[allow(clippy::mut_from_ref)]
unsafe fn writers() -> &'static mut ([ParkedWrite; NUM_PDS], u64) {
    // SAFETY: see `Writers`' Sync impl.
    unsafe { &mut *WRITERS.0.get() }
}

/// Park identity `writer`'s write of the first `words` WORDs of `src` on
/// `target`'s full queue — `e->e_parm = lm; evinsert(e, &p->p_qnq);`
/// (`aes/gemqueue.c:120-124`). The caller then sleeps on
/// [`sched::wait_qnq`](crate::bdos::process::sched::wait_qnq)`(writer)`.
///
/// # Safety
/// Single trap path.
pub unsafe fn park_write(writer: usize, target: usize, src: &[i16], words: usize) {
    // SAFETY: caller upholds the single-trap-path invariant.
    let (tab, next) = unsafe { writers() };
    let Some(w) = tab.get_mut(writer) else { return };
    let words = words.min(MAX_MSG_WORDS).min(src.len());
    *next += 1;
    w.state = WriteState::Parked;
    w.target = target;
    w.seq = *next;
    w.words = words;
    w.msg = [0; MAX_MSG_WORDS];
    w.msg[..words].copy_from_slice(&src[..words]);
}

/// `true` while identity `writer` has an `appl_write` in flight on `p_qnq` —
/// parked, or serviced and not yet collected by [`take_write_done`]. Its
/// re-executed call (S2) must not start a new write.
///
/// # Safety
/// Single trap path.
pub unsafe fn write_in_flight(writer: usize) -> bool {
    // SAFETY: caller upholds the single-trap-path invariant.
    let (tab, _) = unsafe { writers() };
    tab.get(writer).is_some_and(|w| w.state != WriteState::Idle)
}

/// Collect a serviced write: `true` (and the entry is idle again) when a read
/// copied identity `writer`'s parked message into its target's queue —
/// `azombie(e, 1)`, the `1` `ap_rdwr` then returns (`gemqueue.c:117`).
///
/// # Safety
/// Single trap path.
pub unsafe fn take_write_done(writer: usize) -> bool {
    // SAFETY: caller upholds the single-trap-path invariant.
    let (tab, _) = unsafe { writers() };
    match tab.get_mut(writer) {
        Some(w) if w.state == WriteState::Done => {
            *w = ParkedWrite::IDLE;
            true
        }
        _ => false,
    }
}

/// The read side of `aqueue` (`aes/gemqueue.c:104-118`): a read of
/// `length` bytes from identity `target`'s queue `queue` — which held
/// `bytes_before` bytes when the read began — has just completed; service the
/// head of `target`'s `p_qnq`, if any. Every path that reads a queue calls
/// this once per completed read: `appl_read`, `evnt_mesag`, `evnt_multi`'s
/// `MU_MESAG`, and the `appl_exit` / `sh_ldapp` drains.
///
/// * **`ap_rdwr`'s fast path services nothing.** A 16-byte read from a queue
///   holding exactly 16 bytes is done by `ap_rdwr` itself, without `aqueue`
///   (`aes/gemaplib.c:77-82`), so no parked writer is looked at.
/// * Otherwise the head — the most recently parked writer on `target` — has
///   its message enqueued (`doq(!isqwrite, …)`, with `doq`'s redraw/arrow
///   coalescing, `:31-74,116`) and is woken with 1 (`azombie(e, 1)`, `:117`).
///   EmuTOS does this without re-testing room; a head that still does not fit
///   (only possible with a message longer than the one just read) would make
///   `doq` copy past the end of `p_queue` there. NewTOS leaves such a head
///   parked rather than overrun the queue.
///
/// # Safety
/// Single trap path. `queue` is `target`'s own queue.
pub unsafe fn after_read(target: usize, queue: &mut MsgQueue, length: usize, bytes_before: usize) {
    if length == 2 * MSG_WORDS && bytes_before == 2 * MSG_WORDS {
        return; // `ap_rdwr`'s 16-byte fast path (`gemaplib.c:77-82`)
    }
    // SAFETY: caller upholds the single-trap-path invariant.
    let (tab, _) = unsafe { writers() };
    let head = tab
        .iter()
        .enumerate()
        .filter(|(_, w)| w.state == WriteState::Parked && w.target == target)
        .max_by_key(|(_, w)| w.seq)
        .map(|(i, _)| i);
    let Some(i) = head else { return };
    let w = &mut tab[i];
    if !queue.fits(w.words) {
        return;
    }
    let posted = queue.push_words(&w.msg, w.words);
    super::trace::msg_post(super::appl::reported_pid(target), &w.msg, w.words, posted);
    w.state = WriteState::Done;
    crate::bdos::process::sched::wake(crate::bdos::process::sched::wait_qnq(i));
    // The serviced write landed in `target`'s queue like any other write.
    wake_mesg_waiter(target);
}

/// The parked-write half of a slot's release ([`release`]). The writer
/// `idx`'s own parked write goes with it, undelivered. Writes parked on
/// `idx`'s queue are completed with 1 and their messages discarded, as the
/// release discards the messages already in that queue — **NewTOS-only**:
/// EmuTOS's descriptors are never released (its accessories never end and the
/// chain's `D.g_int[0]` outlives every program, `aes/gempd.c:32-35`), so a
/// parked writer there always has a reader left; here the identity is gone and
/// nobody would ever read it (`docs/COMPATIBILITY.md`, A116).
///
/// # Safety
/// Single trap / process-release path.
unsafe fn release_parked_writes(idx: usize) {
    // SAFETY: caller upholds the single-path invariant.
    let (tab, _) = unsafe { writers() };
    if let Some(w) = tab.get_mut(idx) {
        *w = ParkedWrite::IDLE;
    }
    for (i, w) in tab.iter_mut().enumerate() {
        if w.state == WriteState::Parked && w.target == idx {
            w.state = WriteState::Done;
            crate::bdos::process::sched::wake(crate::bdos::process::sched::wait_qnq(i));
        }
    }
}

/// The descriptor of identity `idx`, or `None` for an index outside the
/// table — the only way a lookup can miss, since every identity has its entry
/// from boot (EmuTOS `pd_index`, `aes/gempd.c:32-35`). Callers that hold a
/// **reported** pid go through `appl::identity_of` first.
///
/// # Safety
/// Single trap path.
pub unsafe fn by_index(idx: usize) -> Option<&'static mut Aespd> {
    // SAFETY: single trap path.
    let t = unsafe { table() };
    t.get_mut(idx)
}

/// Find the identity whose descriptor carries the 8-char `name` — EmuTOS
/// `fpdnm(name, …)` (`aes/gempd.c:38-57`): a walk over every entry comparing
/// `p_name`, with no liveness test. Every entry starts blank
/// (`geminit.c:868`) and only [`register`] writes a name, so a program's name
/// is found while — and only while — its slot is the one `register` last
/// named that way (one name per slot; a later occupant overwrites it in
/// place, `aes/gemshlib.c:633`). `None` when no entry carries the name.
///
/// # Safety
/// Single trap path.
pub unsafe fn find_by_name(name: &[u8; 8]) -> Option<usize> {
    // SAFETY: single trap path.
    let t = unsafe { table() };
    t.iter().position(|s| &s.name == name)
}

/// Close the descriptor of identity `idx` — EmuTOS `ap_exit`
/// (`aes/gemaplib.c:251-261`):
///
/// ```c
/// wm_update(BEG_UPDATE);
/// mn_cleanup();
/// wait_for_accs(AP_ACCLOSE);      /* block until all DAs have seen AC_CLOSE */
/// if (rlr->p_qindex)
///     ap_rdwr(MU_MESAG, rlr, rlr->p_qindex, (WORD *)D.g_valstr);
/// wm_update(END_UPDATE);
/// all_run();
/// rlr->p_flags &= ~AP_OPEN;       /* say appl_exit() is done */
/// ```
///
/// Two things happen to the descriptor and no more: **the queue is drained**
/// (the `ap_rdwr(MU_MESAG, …)` reads every pending message into the scratch
/// buffer `D.g_valstr`, i.e. throws it away) and **`AP_OPEN` is cleared**.
/// `p_name` and the remaining flags survive, and the `AESPD` itself is never
/// freed — EmuTOS's descriptors are fixed entries of `D.g_int[]`/`D.g_acc[]`
/// addressed by index (`pd_index`, `aes/gempd.c:32-35`) and `fpdnm` walks
/// that array with no liveness test (`:38-57`), so a message addressed to a
/// pid always finds its descriptor, before and after `appl_exit`. NewTOS's
/// table has the same shape (one entry per identity, from boot), so there is
/// nothing here to free either.
///
/// This matters here and not only in theory: the desktop calls `appl_exit`
/// before every program it launches and `appl_init` when it resumes
/// (`desk/deskmain.c:2166,1953`, A28). Destroying the descriptor here used to
/// take the desktop's message pipe with it when a program quit (A29), and
/// from then on every `WM_REDRAW`/`WM_CLOSED`/`WM_MOVED`/`MN_SELECTED` the AES
/// posted to the desktop was silently dropped.
///
/// `mn_cleanup` (`gemmnlib.c:740-758`) and `wait_for_accs` (`geminit.c:599`)
/// are the desk-accessory half and live one level up, in
/// [`super::appl::appl_exit`]: they have to run **before** this drain and the
/// wait can yield to an accessory, which this function (a leaf) cannot do.
/// The `wm_update` pair around it is `appl_exit`'s too (A118); `all_run`
/// after it is not reproduced.
///
/// # Safety
/// Single trap path.
pub unsafe fn close(idx: usize) {
    // SAFETY: single trap path.
    if let Some(slot) = unsafe { by_index(idx) } {
        // SAFETY: single trap path.
        unsafe { drain(idx, &mut slot.queue) }; // ap_rdwr(MU_MESAG, rlr, rlr->p_qindex, …)
        slot.flags &= !AP_OPEN; // rlr->p_flags &= ~AP_OPEN
    }
}

/// `if (rlr->p_qindex) ap_rdwr(MU_MESAG, rlr, rlr->p_qindex, (WORD *)D.g_valstr);`
/// (`ap_exit`, `aes/gemaplib.c:256-257`; `sh_ldapp`, `aes/gemshlib.c:655`) —
/// read every queued byte into a scratch buffer. That is a read like any
/// other: unless it took `ap_rdwr`'s fast path (exactly one 16-byte message
/// queued) it services the head of `p_qnq` ([`after_read`]), whose message
/// then waits in the emptied queue.
///
/// # Safety
/// Single trap path. `queue` is `idx`'s own queue.
unsafe fn drain(idx: usize, queue: &mut MsgQueue) {
    let had = queue.bytes();
    if queue.pending() == 0 {
        return;
    }
    queue.clear();
    // SAFETY: caller upholds the single-trap-path invariant.
    unsafe { after_read(idx, queue, had, had) };
}

/// `sh_ldapp`'s `AP_OPEN` cleanup (`aes/gemshlib.c:650-657`) — the program
/// that has just terminated called `appl_init()` but never `appl_exit()`:
///
/// ```c
/// if (rlr->p_flags & AP_OPEN) {
///     mn_cleanup();
///     if (rlr->p_qindex) ap_rdwr(MU_MESAG, rlr, rlr->p_qindex, (WORD *)D.g_valstr);
///     rlr->p_flags &= ~AP_OPEN;
/// }
/// ```
///
/// EmuTOS does "the important parts of `appl_exit`" on the program's behalf,
/// because `rlr` (its single AESPD 0) is about to be reused by the next
/// program and a stale `AP_OPEN` plus an undrained queue would corrupt that
/// program's state. Those important parts are the same two the full `ap_exit`
/// performs — drain the queue, clear `AP_OPEN` — so this is [`close`]'s body
/// under EmuTOS's `AP_OPEN` guard. `mn_cleanup` (`gemmnlib.c:740-758`) sends
/// `AC_CLOSE` to every registered desk accessory and frees the submenu buffer;
/// NewTOS has neither.
///
/// Like `ap_exit`, this **keeps the descriptor**: `p_name` stays, so the next
/// program to occupy the slot finds the pd to rename (`aes/gemshlib.c:633`).
/// A descriptor without `AP_OPEN` (the program did call `appl_exit`, or never
/// called `appl_init`) is left alone, as EmuTOS's `if` leaves it alone.
///
/// The kernel launcher calls this at `sh_ldapp`'s point, after the program's
/// `Pterm` has returned. Since the slot's release already ran [`release`] by
/// then, the guard finds `AP_OPEN` clear on that path; the call stays as the
/// `sh_ldapp` step it is, and as the cleanup for any caller that has not
/// released the slot.
///
/// # Safety
/// Single trap path.
pub unsafe fn close_if_open(idx: usize) {
    // SAFETY: single trap path.
    if let Some(slot) = unsafe { by_index(idx) } {
        if slot.flags & AP_OPEN != 0 {
            // `mn_cleanup();` (`gemshlib.c:653`) — AC_CLOSE to every registered
            // desk accessory, for a program that ended without calling
            // `appl_exit` itself. Unlike `ap_exit` this path does **not** wait
            // for `AP_ACCLOSE` (EmuTOS's `sh_ldapp` has no `wait_for_accs`
            // either): the shell loop is about to start the next program and
            // the accessories pick the message up when they next run.
            super::menu::mn_cleanup();
            // SAFETY: single trap path.
            unsafe { drain(idx, &mut slot.queue) }; // ap_rdwr(MU_MESAG, rlr, rlr->p_qindex, …)
            slot.flags &= !AP_OPEN; // rlr->p_flags &= ~AP_OPEN
        }
    }
}

/// The tenant of GEMDOS slot `idx` is gone — called from the process model's
/// slot release (`bdos::process::pterm::release_slot`), for every way a
/// process ends (`Pterm`/`Ptermres`/`Pterm0`, the Ctrl-C terminate, a fault).
/// Flags are cleared and the queue drained; the name stays, as it stays in
/// EmuTOS until the next `p_nameit`.
///
/// This is the union of the two resets EmuTOS applies to its one `rlr` at the
/// launch boundary — `rlr->p_flags = 0` before the next program
/// (`aes/gemshlib.c:636`) and the queue drain after the last one
/// (`:650-657`, `ap_exit` `gemaplib.c:256-257`) — applied where NewTOS's
/// process model learns that a tenant is gone. EmuTOS needs no such hook
/// because a `Pexec` child runs on the parent's `rlr` and the parent sees its
/// leftovers; in NewTOS the child has its own entry (the slot's), and without
/// this reset the slot's **next** tenant inherited a dead program's `AP_OPEN`
/// and unread messages through `appl_init`'s preserve-the-queue rule (A25) —
/// the `handle_pexec` side finding of the A62 report (§8), which runs neither
/// `register` nor `close_if_open` for an EL0-`Pexec`ed child.
///
/// # Safety
/// Single trap / process-release path (no re-entrancy).
pub unsafe fn release(idx: usize) {
    // SAFETY: caller upholds the single-path invariant.
    if let Some(slot) = unsafe { by_index(idx) } {
        slot.queue.clear();
        slot.flags = 0;
    }
    // SAFETY: caller upholds the single-path invariant.
    unsafe { release_parked_writes(idx) };
}

/// Blank the name of identity `idx` — the slot of a desk accessory that has
/// ended ([`super::release_client`], A113). NewTOS-only: EmuTOS's accessories
/// never end, so their names stay for the session (`gempd.c:110-121`); a
/// NewTOS accessory's slot is reused by the next process, which must not
/// answer `appl_find` under the dead accessory's name.
///
/// # Safety
/// Single trap / process-release path (no re-entrancy).
pub unsafe fn clear_name(idx: usize) {
    // SAFETY: caller upholds the single-path invariant.
    if let Some(slot) = unsafe { by_index(idx) } {
        slot.name = [b' '; AP_NAMELEN];
    }
}

/// The 8-char `p_name` length (EmuTOS `aes/struct.h:111`, `AP_NAMELEN`).
const AP_NAMELEN: usize = 8;

/// The filename portion of a launch path — EmuTOS `sh_name` (`aes/gemshlib.c:594`):
/// everything after the last path separator (`\`). A bare filename (no
/// separator) is returned whole.
fn base_name(path: &[u8]) -> &[u8] {
    let start = path
        .iter()
        .rposition(|&c| c == b'\\')
        .map(|i| i + 1)
        .unwrap_or(0);
    &path[start..]
}

/// Fill the 8-char `p_name` `dst` from the program name `src` — EmuTOS
/// `p_nameit` (`aes/gempd.c:77-86`): copy up to [`AP_NAMELEN`] chars, stopping
/// at the first `.` (drop the extension), then space-pad the remainder.
fn p_nameit(dst: &mut [u8; AP_NAMELEN], src: &[u8]) {
    let mut i = 0;
    while i < AP_NAMELEN {
        match src.get(i) {
            Some(&c) if c != 0 && c != b'.' => {
                dst[i] = c;
                i += 1;
            }
            _ => break,
        }
    }
    while i < AP_NAMELEN {
        dst[i] = b' ';
        i += 1;
    }
}

/// Hand the AES process descriptor of identity `idx` over to the program
/// about to run from `path` — EmuTOS `sh_ldapp`'s three lines between finding
/// the program and `dos_exec` (`aes/gemshlib.c:633-636`):
///
/// ```c
/// p_nameit(rlr, fname);
/// p_setappdir(rlr, D.s_cmd);
/// rlr->p_flags = 0;
/// ```
///
/// Two of the three land here. `p_name` becomes the 8-char, space-padded,
/// extensionless program name so a later `appl_find(name)` finds the process
/// ([`find_by_name`] / EmuTOS `fpdnm`, `aes/gempd.c:38-57`), and **`p_flags` is
/// cleared**: the new occupant has not called `appl_init` yet, so none of
/// `AP_OPEN`/`AP_MESAG`/`AP_ACCLOSE` may still be set from the previous one.
/// (`p_setappdir` is the launcher's own step,
/// [`super::resource::set_app_dir`].)
///
/// The flag reset is load-bearing because a GEMDOS slot, like EmuTOS's one
/// `rlr`, is occupied by one program after another and its descriptor is the
/// same entry each time. Its one consumer is `sh_ldapp`'s post-`dos_exec`
/// cleanup ([`close_if_open`], `aes/gemshlib.c:650-657`), whose whole question
/// is "did **this** program call `appl_init` without `appl_exit`?". Without
/// the reset it read the *previous* occupant's `AP_OPEN` and drained a queue
/// that had nothing to do with the program that just ended (A28).
///
/// `p_flags = 0` does **not** touch the queue — draining it is the other half
/// of EmuTOS's boundary, the desktop's own `appl_exit()` in its epilog
/// (`desk/deskmain.c:2166` → `ap_exit`, [`close`]).
///
/// An index outside the table names nothing.
///
/// # Safety
/// Single trap / launcher path (no re-entrancy), same discipline as [`open`].
pub unsafe fn register(idx: usize, path: &[u8]) {
    // SAFETY: caller upholds the single-path invariant.
    if let Some(slot) = unsafe { by_index(idx) } {
        p_nameit(&mut slot.name, base_name(path)); // p_nameit(rlr, fname)
        slot.flags = 0; // rlr->p_flags = 0
    }
}
