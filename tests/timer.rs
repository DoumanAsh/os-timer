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
    assert!(!timer.is_scheduled());
    timer.schedule_once(time::Duration::from_millis(250));
    assert!(timer.is_scheduled());

    std::thread::sleep(time::Duration::from_millis(1000));

    assert_eq!(COUNT.load(Ordering::Acquire), 1);

    #[cfg(all(unix, not(any(target_os = "macos", target_os = "ios"))))]
    assert!(!timer.is_scheduled());

    timer.schedule_once(time::Duration::from_millis(250));
    timer.cancel();

    assert!(!timer.is_scheduled());

    std::thread::sleep(time::Duration::from_millis(1000));
    assert_eq!(COUNT.load(Ordering::Acquire), 1);

    timer.cancel();
    assert!(!timer.is_scheduled());

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
    assert!(!timer.is_scheduled());
    timer.schedule_interval(time::Duration::from_secs(1), time::Duration::from_millis(300));
    assert!(timer.is_scheduled());

    std::thread::sleep(time::Duration::from_millis(1100));
    assert_eq!(COUNT.load(Ordering::Acquire), 1);
    assert!(timer.is_scheduled());
    std::thread::sleep(time::Duration::from_millis(1150));
    assert_eq!(COUNT.load(Ordering::Acquire), 5);
    assert!(timer.is_scheduled());

    timer.cancel();
    assert!(!timer.is_scheduled());

    std::thread::sleep(time::Duration::from_millis(1100));
    assert_eq!(COUNT.load(Ordering::Acquire), 5);
}

#[test]
fn timer_schedule_interval_without_initial() {
    static COUNT: AtomicU8 = AtomicU8::new(0);

    fn cb() {
        COUNT.fetch_add(1, Ordering::AcqRel);
    }

    let timer = Timer::new(Callback::plain(cb)).expect("To create timer");
    assert!(!timer.is_scheduled());
    assert!(timer.schedule().interval(time::Duration::from_millis(200)).schedule());
    assert!(timer.is_scheduled());

    std::thread::sleep(time::Duration::from_millis(150));
    assert_eq!(COUNT.load(Ordering::Acquire), 0);
    assert!(timer.is_scheduled());
    std::thread::sleep(time::Duration::from_millis(150));
    assert_eq!(COUNT.load(Ordering::Acquire), 1);
    assert!(timer.is_scheduled());
    std::thread::sleep(time::Duration::from_millis(1000));
    assert_eq!(COUNT.load(Ordering::Acquire), 6);
    assert!(timer.is_scheduled());

    timer.cancel();
    assert!(!timer.is_scheduled());

    std::thread::sleep(time::Duration::from_secs(1));
    assert_eq!(COUNT.load(Ordering::Acquire), 6);
}
