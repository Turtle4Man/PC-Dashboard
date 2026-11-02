//! System-wide keyboard lock.
//!
//! A low-level keyboard hook swallows key presses while the lock is active, so
//! the focused application never receives them. Key releases are still
//! delivered: a key held when the lock starts must be able to come back up, or
//! Windows would keep it stuck down.
//!
//! The hook runs on its own thread with a message loop, which Windows requires
//! for `WH_KEYBOARD_LL`, and it is removed as soon as the lock is lifted. Apart
//! from a lock, no keystroke goes through this process.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock, mpsc};
use std::thread;
use std::time::Duration;

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, MSG, PM_NOREMOVE, PeekMessageW, PostThreadMessageW,
    SetWindowsHookExW, UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_QUIT, WM_SYSKEYDOWN,
};

/// Bounds for the automatic unlock delay. The interface only offers values
/// inside this range; the clamp keeps a malformed request harmless.
const MIN_HOLD: Duration = Duration::from_secs(5);
const MAX_HOLD: Duration = Duration::from_secs(3600);

/// How long the hook thread may take to install the hook.
const HOOK_START_TIMEOUT: Duration = Duration::from_secs(2);

static LOCKED: AtomicBool = AtomicBool::new(false);

/// Identifies the pending automatic unlock. Every manual change bumps it, which
/// turns an older timer into a no-op.
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Thread id of the hook thread while the hook is installed.
static HOOK_THREAD: OnceLock<Mutex<Option<u32>>> = OnceLock::new();

pub fn is_locked() -> bool {
    LOCKED.load(Ordering::SeqCst)
}

/// Locks or unlocks the keyboard. `hold` bounds how long the lock lasts: when
/// it elapses the keyboard unlocks itself and `on_unlock` is called with
/// `false`. `None` keeps the keyboard locked until it is released manually.
pub fn set_locked(
    locked: bool,
    hold: Option<Duration>,
    on_unlock: impl Fn(bool) + Send + 'static,
) -> Result<bool, String> {
    if !locked {
        release();
        return Ok(false);
    }

    install_hook()?;
    LOCKED.store(true, Ordering::SeqCst);

    // Bumping the generation also disarms the timer of a previous lock, so an
    // unlimited lock cannot be lifted by a duration chosen earlier.
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    if let Some(hold) = hold {
        let timer = thread::Builder::new()
            .name("keyboard-unlock".into())
            .spawn(move || {
                thread::sleep(hold.clamp(MIN_HOLD, MAX_HOLD));
                // A manual unlock, or a newer lock, bumped the generation: this
                // timer is stale and must leave the keyboard alone.
                if GENERATION
                    .compare_exchange(
                        generation,
                        generation + 1,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    )
                    .is_ok()
                {
                    LOCKED.store(false, Ordering::SeqCst);
                    remove_hook();
                    on_unlock(false);
                }
            });

        if let Err(error) = timer {
            release();
            return Err(format!(
                "Impossible de planifier le déverrouillage : {error}"
            ));
        }
    }

    Ok(true)
}

fn release() {
    GENERATION.fetch_add(1, Ordering::SeqCst);
    LOCKED.store(false, Ordering::SeqCst);
    remove_hook();
}

fn hook_thread() -> &'static Mutex<Option<u32>> {
    HOOK_THREAD.get_or_init(|| Mutex::new(None))
}

fn install_hook() -> Result<(), String> {
    let mut slot = hook_thread()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if slot.is_some() {
        return Ok(());
    }

    let (sender, receiver) = mpsc::channel();
    thread::Builder::new()
        .name("keyboard-hook".into())
        .spawn(move || run_hook_thread(&sender))
        .map_err(|error| format!("Impossible de démarrer le verrou clavier : {error}"))?;

    match receiver.recv_timeout(HOOK_START_TIMEOUT) {
        Ok(Ok(thread_id)) => {
            *slot = Some(thread_id);
            Ok(())
        }
        Ok(Err(error)) => Err(error),
        Err(_) => Err("Le verrou clavier ne répond pas.".into()),
    }
}

fn remove_hook() {
    let mut slot = hook_thread()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if let Some(thread_id) = slot.take() {
        // The hook thread unhooks itself and exits when it receives WM_QUIT.
        unsafe {
            let _ = PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
}

fn run_hook_thread(sender: &mpsc::Sender<Result<u32, String>>) {
    let mut message = MSG::default();

    // Create the message queue before reporting readiness, so the hook can be
    // removed with PostThreadMessageW as soon as the lock is lifted.
    unsafe {
        let _ = PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE);
    }

    let hook = match unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(swallow_keys), None, 0) } {
        Ok(hook) => hook,
        Err(error) => {
            let _ = sender.send(Err(format!(
                "Windows a refusé le verrouillage du clavier : {error}"
            )));
            return;
        }
    };

    let _ = sender.send(Ok(unsafe { GetCurrentThreadId() }));

    // The hook only fires while this thread pumps messages.
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {}

    unsafe {
        let _ = UnhookWindowsHookEx(hook);
    }
}

/// Drops key presses while the lock is active. `code` is negative for events
/// the hook must hand over untouched.
unsafe extern "system" fn swallow_keys(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0
        && LOCKED.load(Ordering::SeqCst)
        && matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN)
    {
        return LRESULT(1);
    }

    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}
