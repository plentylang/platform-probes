# Plenty platform probes

Small, standalone experiments for Plenty's native macOS ARM64 and Windows x86-64 ports. Pushes run on native GitHub Actions runners, with Linux x86-64 as a control. This repository does not build or qualify the Plenty compiler.

## Run

Install Rust 1.99.1 and a native C toolchain (Linux C compiler, Xcode command-line tools, or Visual Studio C++ build tools with the Windows SDK).

```sh
cargo run --locked -- artifacts
```

The program generates an object using Cranelift 0.137.0, compiles a C harness, links it with that object, and executes assertions. A failed compilation, link, or assertion fails the job. No emulator or cross-compiler is used in CI. The workflow verifies the Rust host triple before running.

## Coverage

- Native calls from C into generated code, including ten integer arguments spilling to the stack.
- Mixed integer/floating-point arguments and narrow signed/unsigned arguments.
- Generated code calling a C function.
- A native entry adapter calling a function with Cranelift's internal `Tail` calling convention.

The C harness uses the platform compiler: the Windows job exercises MSVC, and macOS uses the Apple toolchain. This is an initial ABI smoke test, not comprehensive ABI conformance. Aggregates, 128-bit values, variadics, exception/unwind handling, actor stack switches, runtime services, minimum OS versions, and Plenty's dependency engines remain unqualified.

## CI evidence

[Native ABI probes](https://github.com/plentylang/platform-probes/actions/workflows/native.yml) runs on `macos-15` (ARM64), `windows-2025` (x86-64 MSVC), and `ubuntu-24.04` (x86-64). Each job saves its generated object, executable, toolchain/host details, and probe log as an artifact. The Cargo lockfile and exact Cranelift versions keep the experiment reproducible; runner images may evolve.

Runner labels are documented in [GitHub's hosted runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).
