use libc::{
    pthread_create, pthread_kill, pthread_self, pthread_t, sigaction, sigemptyset, SA_RESTART,
    SA_SIGINFO, SIGPROF,
};
use magnus::{function, prelude::*, Error, Ruby};
use rb_sys::VALUE;
use std::mem::zeroed;
use std::ptr;
use std::thread::sleep;
use std::time::Duration;
use std::sync::Mutex;
use lazy_static::lazy_static;

const MAX_STACK_DEPTH: usize = 2048;

lazy_static! {
    static ref BUFFER: Mutex<[VALUE; MAX_STACK_DEPTH]> = Mutex::new([0 as VALUE; MAX_STACK_DEPTH]);
    static ref LINES: Mutex<[i32; MAX_STACK_DEPTH]> = Mutex::new([0; MAX_STACK_DEPTH]);
}

extern "C" fn signal_handler(_: i32, _: *mut libc::siginfo_t, _: *mut libc::c_void) {
    // let current_thread = get_current_thread_id();
    // println!("Signal received {:?}", current_thread);
    // let mut buffer = BUFFER.lock().unwrap();
    // let mut lines = LINES.lock().unwrap();

    // unsafe { rb_profile_frames(0, MAX_STACK_DEPTH as i32, buffer.as_mut_ptr(), lines.as_mut_ptr()) };
}

fn setup_signal_handler() {
    unsafe {
        let mut sa: sigaction = zeroed();
        sa.sa_sigaction = signal_handler as usize;
        sa.sa_flags = SA_RESTART | SA_SIGINFO;
        sigemptyset(&mut sa.sa_mask);
        sigaction(SIGPROF, &sa, std::ptr::null_mut());
    }
}

extern "C" fn scheduler_func(thread: *mut libc::c_void) -> *mut libc::c_void {
    loop {
        sleep(Duration::from_millis(1));
        unsafe {
            pthread_kill(thread as pthread_t, SIGPROF);
        }
    }
}

fn get_current_thread_id() -> pthread_t {
    unsafe { pthread_self() }
}

fn start_scheduler() {
    unsafe {
        let mut thread: pthread_t = zeroed();
        let current_thread = get_current_thread_id();

        pthread_create(
            &mut thread,
            ptr::null(),
            scheduler_func,
            current_thread as *mut libc::c_void,
        );
    }
}

extern "C" fn star_thread_func(_: *mut libc::c_void) -> *mut libc::c_void {
    start_scheduler();
    sleep(Duration::from_secs(60 * 60));
    ptr::null_mut()
}

fn start_thread() {
    unsafe {
        let mut thread: pthread_t = zeroed();

        pthread_create(
            &mut thread,
            ptr::null(),
            star_thread_func,
            std::ptr::null_mut(),
        );
    }
}

fn sleep_with_gvl() {
    sleep(Duration::from_secs(60 * 60));
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("SdbSignal")?;
    module.define_singleton_method("setup_signal_handler", function!(setup_signal_handler, 0))?;
    module.define_singleton_method("start_scheduler", function!(start_scheduler, 0))?;
    module.define_singleton_method("sleep_with_gvl", function!(sleep_with_gvl, 0))?;
    module.define_singleton_method("start_thread", function!(start_thread, 0))?;

    Ok(())
}
