use libc::{
    pthread_create, pthread_kill, pthread_self, pthread_t, sigaction, sigemptyset, SA_RESTART,
    SA_SIGINFO, SIGPROF,
};
use magnus::{function, prelude::*, Error, Ruby};
use rb_sys::{rb_profile_frames, VALUE};
use std::mem::zeroed;
use std::ptr;
use std::thread::sleep;
use std::time::Duration;

extern "C" fn signal_handler(_: i32, _: *mut libc::siginfo_t, _: *mut libc::c_void) {
    let mut buff: [VALUE; 10] = [0 as VALUE; 10];
    let mut lines: [i32; 10] = [0; 10];
    let frames = unsafe { rb_profile_frames(0, 10, buff.as_mut_ptr(), lines.as_mut_ptr()) };

    println!("Signal received! Number of frames: {}", frames);
    for i in 0..frames as usize {
        println!("Frame {}: VALUE = {:?}, Line = {}", i, buff[i], lines[i]);
    }
    println!("Signal received!");
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
        sleep(Duration::from_millis(1000));
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

fn sleep_with_gvl() {
    sleep(Duration::from_secs(60 * 10));
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("SdbSignal")?;
    module.define_singleton_method("setup_signal_handler", function!(setup_signal_handler, 0))?;
    module.define_singleton_method("start_scheduler", function!(start_scheduler, 0))?;
    module.define_singleton_method("sleep_with_gvl", function!(sleep_with_gvl, 0))?;

    Ok(())
}
