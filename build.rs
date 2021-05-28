fn main() {
    #[cfg(all(target_family = "unix", not(any(target_os = "macos", target_os = "ios"))))]
    {
        cc::Build::new().file("src/timer/posix.c").compile("os-timer-posix-c.a");
    }
}
