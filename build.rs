fn is_unix(target: &str) -> bool {
    target.contains("linux") ||
    target.contains("freebsd") ||
    target.contains("netbsd") ||
    target.contains("openbsd") ||
    target.contains("dragonfly") ||
    target.contains("haiku") ||
    target.contains("vxworks") ||
    target.contains("solaris")
}

fn main() {
    use std::env;

    let target = env::var("TARGET").unwrap();

    if is_unix(&target) {
        cc::Build::new().file("src/timer/posix.c").compile("os-timer-posix-c.a");
    }
}
