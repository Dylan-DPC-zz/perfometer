#![feature(no_more_cas)]

pub mod registry;
use std::sync::atomic::{AtomicU64, Ordering};
use std::fmt::Debug;
use libc::{clock_gettime, CLOCK_MONOTONIC, timespec, c_int};
use std::convert::{TryFrom, TryInto};

#[derive(Clone, Debug, Default)]
pub struct Header {
    name: String,
}

pub trait Counter: Reset + Debug {
    fn begin(&self) {}
    fn increment(&self) {}
    fn end(&self) {}
    fn set_elapsed(&self, _elapsed: u64) {}
    fn set_count(&self, _count: u64) {}
    fn cancel(&self) {}
    fn reset(&mut self) { self.reset_it() }

}

pub trait Reset {
    fn reset_it(&mut self);
}

impl<T> Reset for T where T: Default {
    fn reset_it(&mut self) {
        *self = T::default();
    }
}

#[derive(Debug, Default)]
pub struct EventCounter {
    headers: Header,
    event_count: AtomicU64
}

impl Counter for EventCounter {
    fn increment(&self, value: u64) {
        self.event_count.fetch_add(value, Ordering::SeqCst);
    }
    fn set_count(&self, count: u64) { self.event_count.store(count, Ordering::SeqCst); }
}

#[derive(Debug, Default)]
pub struct ElapsedCounter {
    headers: Header,
    event_count: AtomicU64,
    time_start: AtomicU64,
    time_total: AtomicU64,
    time_least: AtomicU64,
    time_most: AtomicU64,
}

impl Counter for ElapsedCounter {
    fn begin(&self) {
        let mut ts = default_timespec();
        unsafe {
            let _ = get_time(&mut ts);
            self.time_start.store(ts.tv_sec.try_into().unwrap(), Ordering::SeqCst);
        }
    }

    fn end(&self) {
        loop {
            let time_start = self.time_start.load(Ordering::SeqCst);
            if time_start > 0 {
                let mut ts = default_timespec();
                let _ = unsafe { get_time(&mut ts)};
                let elapsed: u64 = u64::try_from(ts.tv_sec).unwrap() - time_start;
                if self.time_start.compare_and_swap(time_start, 0, Ordering::SeqCst) != time_start {
                    continue;
                }

                let time_least = self.time_least.load(Ordering::SeqCst);
                if time_least == 0 || time_least > elapsed {
                    self.event_count.fetch_add(1u64, Ordering::SeqCst);
                    self.time_total.fetch_add(elapsed, Ordering::SeqCst);
                }

                let time_most = self.time_most.load(Ordering::SeqCst);
                if time_most < elapsed {
                    if self.time_most.compare_and_swap(time_most, elapsed, Ordering::SeqCst) != time_most {
                        continue;
                    }
                }

                break;
            }
        }

        loop {
            let time_start = self.time_start.load(Ordering::SeqCst);
            if time_start > 0 {
                let mut ts = default_timespec();
                let _ = unsafe { get_time(&mut ts)};
                let elapsed: u64 = u64::try_from(ts.tv_sec).unwrap() - time_start;
                self.time_start.store(0, Ordering::SeqCst);

                loop {
                    let time_least = self.time_least.load(Ordering::SeqCst);
                    if time_least == 0 || time_least > elapsed {
                        self.event_count.fetch_add(1u64, Ordering::SeqCst);
                        self.time_total.fetch_add(elapsed, Ordering::SeqCst);
                        break;
                    }
                }

                loop {
                    let time_most = self.time_most.load(Ordering::SeqCst);
                    if time_most < elapsed {
                        self.time_most.store(elapsed, Ordering::SeqCst);
                        break;
                    }
                }
                break;
            }
        }
    }

    fn cancel(&self) {
        self.time_start.store(0, Ordering::SeqCst);
    }
}

#[derive(Debug, Default)]
pub struct IntervalCounter {
    headers: Header,
    event_count: AtomicU64,
    time_event: AtomicU64,
    time_first: AtomicU64,
    time_last: AtomicU64,
    time_least: AtomicU64,
    time_most: AtomicU64,
}

impl Counter for IntervalCounter {
    fn increment(&self) {
        let mut ts = default_timespec();
        let now = u64::try_from(unsafe { get_time(&mut ts) }).unwrap();

        loop {
            let count = self.event_count.load(Ordering::SeqCst);
            match count {
                0 => self.time_first.store(now, Ordering::SeqCst),
                1 => {
                    self.time_least.store(now - self.time_last.load(Ordering::SeqCst), Ordering::SeqCst);
                    self.time_most.store(now - self.time_last.load(Ordering::SeqCst), Ordering::SeqCst);
                    break;
                },
                _ => {
                    let interval = now - self.time_last.load(Ordering::SeqCst);
                    if interval < self.time_least.load(Ordering::SeqCst) {
                        self.time_least.store(interval, Ordering::SeqCst);
                    }
                    if interval > self.time_most.load(Ordering::SeqCst) {
                        self.time_most.store(interval, Ordering::SeqCst);
                    }
                    break;
                }
            };
        }

        self.time_last.store(now, Ordering::SeqCst);
        self.event_count.fetch_add(1, Ordering::SeqCst);

    }
}

pub unsafe fn get_time(timespec: *mut timespec) -> c_int {
    clock_gettime(CLOCK_MONOTONIC, timespec)
}

fn default_timespec() -> timespec {
    timespec { tv_sec: 0i64, tv_nsec: 0i64}
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
