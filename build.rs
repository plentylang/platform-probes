fn main() {
    println!(
        "cargo:rustc-env=PROBE_TARGET={}",
        std::env::var("TARGET").unwrap()
    );
}
