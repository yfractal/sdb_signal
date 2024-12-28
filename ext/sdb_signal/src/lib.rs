use lazy_static::lazy_static;
use libc::{
    c_char, pthread_create, pthread_kill, pthread_self, pthread_t, sigaction, sigemptyset,
    SA_RESTART, SA_SIGINFO, SIGPROF,
};
use magnus::{function, prelude::*, Error, Ruby};
use rb_sys::{rb_define_module, rb_define_singleton_method, Qtrue, VALUE};
use std::mem::zeroed;
use std::ptr;
use std::sync::Mutex;
use std::thread::sleep;
use std::time::Duration;

const MAX_STACK_DEPTH: usize = 2048;

lazy_static! {
    static ref BUFFER: Mutex<[VALUE; MAX_STACK_DEPTH]> = Mutex::new([0 as VALUE; MAX_STACK_DEPTH]);
    static ref LINES: Mutex<[i32; MAX_STACK_DEPTH]> = Mutex::new([0; MAX_STACK_DEPTH]);
}

struct SchedulerData {
    thread: pthread_t,
    rb_threads: VALUE,
}

extern "C" fn signal_handler(_: i32, _: *mut libc::siginfo_t, _: *mut libc::c_void) {
    let current_thread = get_current_thread_id();
    println!("Signal received {:?}", current_thread);

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

extern "C" fn scheduler_func(data_ptr: *mut libc::c_void) -> *mut libc::c_void {
    unsafe {
        let scheduler_data = Box::from_raw(data_ptr as *mut SchedulerData);
        let thread = scheduler_data.thread;

        loop {
            sleep(Duration::from_millis(1));
            pthread_kill(thread as pthread_t, SIGPROF);
        }
    }
}

fn get_current_thread_id() -> pthread_t {
    unsafe { pthread_self() }
}

unsafe extern "C" fn start_scheduler_for_current_thread(_module: VALUE, threads: VALUE) -> VALUE {
    let data = Box::new(SchedulerData {
        thread: get_current_thread_id(),
        rb_threads: threads,
    });

    let data_ptr = Box::into_raw(data) as *mut libc::c_void;

    unsafe {
        let mut thread: pthread_t = zeroed();

        pthread_create(&mut thread, ptr::null(), scheduler_func, data_ptr);
    }

    Qtrue as VALUE
}

fn sleep_with_gvl() {
    sleep(Duration::from_secs(60 * 60));
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("SdbSignal")?;
    module.define_singleton_method("setup_signal_handler", function!(setup_signal_handler, 0))?;
    module.define_singleton_method("sleep_with_gvl", function!(sleep_with_gvl, 0))?;

    unsafe {
        let m = rb_define_module("SdbSignal\0".as_ptr() as *const c_char);

        let start_scheduler_for_current_thread_callback =
            std::mem::transmute::<
                unsafe extern "C" fn(VALUE, VALUE) -> VALUE,
                unsafe extern "C" fn() -> VALUE,
            >(start_scheduler_for_current_thread);

        rb_define_singleton_method(
            m,
            "start_scheduler_for_current_thread\0".as_ptr() as _,
            Some(start_scheduler_for_current_thread_callback),
            1,
        );
    };

    Ok(())
}
