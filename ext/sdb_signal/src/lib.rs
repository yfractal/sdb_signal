mod buffer;

use lazy_static::lazy_static;
use libc::{
    c_char, pthread_create, pthread_kill, pthread_self, pthread_t, sigaction, sigemptyset,
    SA_RESTART, SA_SIGINFO, SIGPROF,
};
use magnus::{function, prelude::*, Error, Ruby};
use rb_sys::{
    rb_define_module, rb_define_singleton_method, rb_int2inum, rb_profile_frame_full_label,
    rb_profile_frame_path, rb_profile_thread_frames, Qtrue, RARRAY_LEN, VALUE,
};
use std::mem::zeroed;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, RwLock};
use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::buffer::*;

const MAX_STACK_DEPTH: usize = 2048;
const ONE_MILLISECOND_NS: u64 = 1_000_000; // 1ms in nanoseconds

lazy_static! {
    static ref BUFFER: Mutex<[VALUE; MAX_STACK_DEPTH]> = Mutex::new([0 as VALUE; MAX_STACK_DEPTH]);
    static ref LINES: Mutex<[i32; MAX_STACK_DEPTH]> = Mutex::new([0; MAX_STACK_DEPTH]);
    static ref ISEQ_BUFFER: Mutex<Buffer> = Mutex::new(Buffer::new());
    static ref COUNTER: AtomicUsize = AtomicUsize::new(0);
    static ref SAMPLING_INTERVAL_NS: AtomicUsize = AtomicUsize::new(ONE_MILLISECOND_NS as usize);
}

struct SchedulerData {
    thread: pthread_t,
    rb_threads: VALUE,
    threads: Vec<pthread_t>,
}

lazy_static! {
    static ref SCHEDULER_DATA: RwLock<SchedulerData> = RwLock::new(SchedulerData {
        thread: 0,
        rb_threads: 0 as VALUE,
        threads: vec![],
    });
}

#[inline]
pub fn arvg_to_ptr(val: &[VALUE]) -> *const VALUE {
    val as *const [VALUE] as *const VALUE
}

fn get_counter_value() -> usize {
    COUNTER.load(Ordering::SeqCst)
}

// stack_scanner fetches all frames through `rb_profile_thread_frames`
// and queries the frame's full label and frame path.
// But, it does not log this data.
unsafe extern "C" fn stack_scanner(_: i32, _: *mut libc::siginfo_t, _: *mut libc::c_void) {
    COUNTER.fetch_add(1, Ordering::SeqCst);
    if let Ok(data) = SCHEDULER_DATA.read() {
        let threads_count = RARRAY_LEN(data.rb_threads) as isize;
        let mut i = 0;
        let mut iseq_buffer = ISEQ_BUFFER.lock().unwrap();
        while i < threads_count {
            let mut buffer = BUFFER.lock().unwrap();
            let mut lines: std::sync::MutexGuard<'_, [i32; 2048]> = LINES.lock().unwrap();

            let argv = &[rb_int2inum(i)];
            let rb_thread = rb_sys::rb_ary_aref(1, arvg_to_ptr(argv), data.rb_threads);

            let frames_count = rb_profile_thread_frames(
                rb_thread,
                0,
                MAX_STACK_DEPTH as i32,
                buffer.as_mut_ptr(),
                lines.as_mut_ptr(),
            );

            let mut j = 0;
            // println!("frames_count={frames_count}");

            while j < frames_count {
                let frame = buffer[j as usize];
                iseq_buffer.push(frame);
                // rb_profile_frame_full_label(frame as VALUE); // mainly cost
                // rb_profile_frame_path(frame as VALUE);

                j += 1
            }

            i += 1;
        }

        iseq_buffer.push_seperator();
    }
}

fn setup_signal_handler() {
    unsafe {
        let mut sa: sigaction = zeroed();
        sa.sa_sigaction = stack_scanner as usize;
        sa.sa_flags = SA_RESTART | SA_SIGINFO;
        sigemptyset(&mut sa.sa_mask);
        sigaction(SIGPROF, &sa, std::ptr::null_mut());
    }
}

extern "C" fn scheduler_func(_: *mut libc::c_void) -> *mut libc::c_void {
    unsafe {
        loop {
            let interval = SAMPLING_INTERVAL_NS.load(Ordering::Relaxed) as u64;

            if interval >= ONE_MILLISECOND_NS {
                sleep(Duration::from_nanos(interval));
            } else {
                // For sub-millisecond intervals, use busy loop as on linux or macos, they can't sleep for less than 1ms
                let start = Instant::now();
                let target = start + Duration::from_nanos(interval);

                while Instant::now() < target {
                    std::hint::spin_loop();
                }
            }

            let data = SCHEDULER_DATA.read().unwrap();
            for thread in &data.threads {
                pthread_kill(*thread as pthread_t, SIGPROF);
            }
        }
    }
}

fn get_current_thread_id() -> pthread_t {
    unsafe { pthread_self() }
}

// start_scheduler creates a new thread through pthread_create which triggers stack scanning every millisecond.
// threads: the threads to scan
unsafe extern "C" fn start_scheduler(_module: VALUE, threads: VALUE) -> VALUE {
    if let Ok(mut data) = SCHEDULER_DATA.write() {
        data.thread = get_current_thread_id();
        data.rb_threads = threads;
    }

    unsafe {
        let mut thread: pthread_t = zeroed();

        pthread_create(
            &mut thread,
            ptr::null(),
            scheduler_func,
            std::ptr::null_mut(),
        );
    }

    Qtrue as VALUE
}

fn sleep_with_gvl() {
    sleep(Duration::from_secs(60 * 60));
}

fn print_counter() {
    println!("counter={}", get_counter_value());
}

unsafe extern "C" fn register_thread(_module: VALUE, threads: VALUE) -> VALUE {
    let thread = get_current_thread_id();
    if let Ok(mut data) = SCHEDULER_DATA.write() {
        println!("push thread={thread}");
        data.rb_threads = threads;
        data.threads.push(thread);
    }

    Qtrue as VALUE
}

fn set_sampling_interval(nanos: usize) {
    SAMPLING_INTERVAL_NS.store(nanos, Ordering::Relaxed);
}

fn get_sampling_interval() -> usize {
    SAMPLING_INTERVAL_NS.load(Ordering::Relaxed)
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("SdbSignal")?;
    module.define_singleton_method("setup_signal_handler", function!(setup_signal_handler, 0))?;
    module.define_singleton_method("sleep_with_gvl", function!(sleep_with_gvl, 0))?;
    module.define_singleton_method("print_counter", function!(print_counter, 0))?;
    module.define_singleton_method("set_sampling_interval", function!(set_sampling_interval, 1))?;
    module.define_singleton_method("get_sampling_interval", function!(get_sampling_interval, 0))?;

    unsafe {
        let m = rb_define_module("SdbSignal\0".as_ptr() as *const c_char);

        let start_scheduler_callback = std::mem::transmute::<
            unsafe extern "C" fn(VALUE, VALUE) -> VALUE,
            unsafe extern "C" fn() -> VALUE,
        >(start_scheduler);

        rb_define_singleton_method(
            m,
            "start_scheduler\0".as_ptr() as _,
            Some(start_scheduler_callback),
            1,
        );

        let register_thread_callback = std::mem::transmute::<
            unsafe extern "C" fn(VALUE, VALUE) -> VALUE,
            unsafe extern "C" fn() -> VALUE,
        >(register_thread);

        rb_define_singleton_method(
            m,
            "register_thread\0".as_ptr() as _,
            Some(register_thread_callback),
            1,
        );
    };

    Ok(())
}
