// In addition to the following, we also have .cargo/config.toml. If your platform is other than
// Linux, then
// - find out what to put in .cargo/config.toml, and
// - share it (by opening a GitHub issue, or even better, by a Pull Request), please.
#![no_std]
#![no_main]

#[cfg(target_os = "linux")]
use core::arch::asm;

use core::future::Future;
use core::hint;
use core::mem::MaybeUninit;
use core::pin::{pin, Pin};
use core::task::{Context, RawWaker, RawWakerVTable, Waker};

#[cfg(all(debug_assertions, target_os = "linux"))]
compile_error!("Build in release profile only. Required by `panic_never`.");

#[cfg(all(debug_assertions, not(target_os = "linux")))]
compile_error!(
    "(1) Build in release profile only. Required by `panic_never`. (2) You are not on Linux, so \
     you may need to tweak .cargo/config.toml. Please do report any fixes. (3) You are not on \
     Linux, so when you run the build (release) binary, it will run indefinitely. Use Ctrl+C or \
     similar to stop it."
);

#[cfg(not(debug_assertions))]
use panic_never as _;

// This is NOT being used - because in debug mode we get the compile error from above. However, we
// do include this in debug mode, otherwise (in debug mode) we would also get a confusing error:
//
// `#[panic_handler]` function required, but not found
#[cfg(debug_assertions)]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

/// As per https://github.com/rust-lang/libc/discussions/5683, we don't use `libc::exit(..)`.
///
/// Neither we use `exit-no-std` (which itself on systems other than DOS/Windows uses
/// `libc::exit(..)`).
///
/// Neither we use `rustix` (which supposedly does use a syscall rather than `libc` on Linux, but
/// it uses `libc` on FreeBSD/NetBSD, even though those two BSD unixes (unlike OpenBSD) _do_ allow
/// direct invocation of syscalls).
///
/// It most likely _is_ possible to use `libc::exit(..)` (directly, or through `exit-no-std` or
/// `rustix`) in `no_std` and `no_main` and with `panic-never`, but with much more boiler plate. See
/// https://github.com/rust-lang/libc/discussions/5683.
#[cfg(target_os = "linux")]
fn exit(status: i32) -> ! {
    unsafe {
        asm!(
            "syscall",
            in("rax") 60,
            in("rdi") status,
            options(noreturn)
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let future = async {
        outer().await;
    };
    if core::hint::black_box(false) {
        //new_dummy_runtime_and_poll_1(future);
        new_dummy_runtime_and_poll_2(future);
    }

    #[cfg(target_os = "linux")]
    exit(0);
    #[cfg_attr(target_os = "linux", allow(unreachable_code))]
    loop {}
}

fn new_dummy_runtime_and_poll_1<F>(fut: F)
where
    F: Future,
{
    let fut_pinned = pin!(fut);
    // We _cannot_ use loop {} to yield dummy value here. Otherwise the compiler optimizes away the
    // rest of the code - and we don't want that, because then we don't get the linking failure that
    // we _do_ want to get (if there is a panic used and that panic is reachable when run by a
    // normal async runtime)!
    //
    //let mut context = loop {};
    let mut context = unsafe {
        #[allow(invalid_value)]
        MaybeUninit::uninit().assume_init()
    };

    let _ = fut_pinned.poll(&mut context);
}
//-------

fn dummy_raw_waker() -> RawWaker {
    unsafe fn clone(_: *const ()) -> RawWaker {
        dummy_raw_waker()
    }
    unsafe fn wake(_: *const ()) {}
    unsafe fn wake_by_ref(_: *const ()) {}
    unsafe fn drop(_: *const ()) {}

    let vtable = &RawWakerVTable::new(clone, wake, wake_by_ref, drop);
    RawWaker::new(hint::black_box(core::ptr::null()), vtable)
}

pub fn dummy_waker() -> Waker {
    //@TODO docs that we don't use loop {}
    //
    // We don't use: unsafe{ #[allow(invalid_value)] MaybeUninit::uninit().assume_init() }
    unsafe { Waker::from_raw(dummy_raw_waker()) }
}

pub fn new_dummy_runtime_and_poll_2<F>(mut future: F)
where
    F: Future,
{
    // Prevent aggressive constant folding / dead code stripping
    let mut pinned = unsafe { Pin::new_unchecked(&mut future) };
    let waker = hint::black_box(dummy_waker());
    let mut cx = Context::from_waker(&waker);

    // Force evaluation of the poll function pointer
    let _ = hint::black_box(pinned.as_mut().poll(&mut cx));

    // Retain ownership reference in black_box
    hint::black_box(future);
}
//-------

async fn outer() {
    inner().await
}

async fn awaited_panic_but_never_reached() {
    panic!()
}

async fn never_awaited_panic() {
    // the following _is_ .await-ed on; but the current function is _never_ .await-ed on.
    awaited_panic_but_never_reached().await;
    panic!()
}

async fn inner() {
    // intentionally no .await
    let _ = core::hint::black_box(never_awaited_panic());

    // @TODO
    if hint::black_box(true) {
        //panic!()
    }
    // @TODO The following exit(anything) DOES MAKE the binary involve panic
    //
    exit(0);
}
