# randomprime

the patcher for metroid prime 1 in randovania

`master` - dirty fork of `aprilwade/randomprime`

`randovania` - based off master, developed into the version used by [randovania](https://github.com/randovania/randovania) via [py-randomprime](https://github.com/randovania/py-randomprime)

# Compiling

1. Install a Rust compiler. It is recommended to use [rustup](https://www.rust-lang.org/tools/install).
2. Add `powerpc-unknown-linux-gnu` as a target, like so: `rustup target add --toolchain stable powerpc-unknown-linux-gnu`
3. Clone the repo and all its submodules: `git clone https://github.com/toasterparty/randomprime --recursive`
4. Run `cargo build`

That should create a standalone executable in `./randomprime/target/debug/randomprime_patcher.exe`.

