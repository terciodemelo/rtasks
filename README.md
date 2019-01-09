# rtasks - a Rust terminal based task management application

# Installation

You can either download the desired version in the releases GitHub page or build yourself as described in the next session

### But the first release has not yet been pushed!! So you have to build it your self

# Build Yourself

To build RTasks yourself, you must be using Rust nightly, then:

## Unoptimized build
```
cargo build -Z unstable-options --out-dir <OUTPUT_DIR>
```

## Release build
```
cargo build -Z unstable-options --release --out-dir <OUTPUT_DIR>
```