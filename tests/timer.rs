use os_timer::{Callback, Timer};

use core::time;
use core::sync::atomic::{AtomicU8, Ordering};

#[test]
fn timer_schedule_once() {
    static COUNT: AtomicU8 = AtomicU8::new(0);

    let cb = || {
        COUNT.fetch_add(1, Ordering::AcqRel);
    };

    let timer = Timer::new(Callback::closure(cb)).expect("To create timer");
    timer.schedule_once(time::Duration::from_millis(250));

    std::thread::sleep(time::Duration::from_millis(1000));

    assert_eq!(COUNT.load(Ordering::Acquire), 1);

    timer.schedule_once(time::Duration::from_millis(250));
    timer.cancel();

    std::thread::sleep(time::Duration::from_millis(1000));
    assert_eq!(COUNT.load(Ordering::Acquire), 1);
}

#[test]
fn timer_schedule_interval() {
    static COUNT: AtomicU8 = AtomicU8::new(0);

    fn cb() {
        COUNT.fetch_add(1, Ordering::AcqRel);
    }

    let timer = Timer::new(Callback::plain(cb)).expect("To create timer");
    timer.schedule_interval(time::Duration::from_secs(1), time::Duration::from_millis(300));

    std::thread::sleep(time::Duration::from_millis(1100));
    assert_eq!(COUNT.load(Ordering::Acquire), 1);
    std::thread::sleep(time::Duration::from_millis(1100));
    assert_eq!(COUNT.load(Ordering::Acquire), 5);

    timer.cancel();

    std::thread::sleep(time::Duration::from_millis(1100));
    assert_eq!(COUNT.load(Ordering::Acquire), 5);
}
