SCRust
======
An open-source, WIP re-implementation of the Starcraft: Broodwar engine in Rust.

SCRust loads the original Starcraft assets, so you need to have an installation of Starcraft on your computer. We recommend to use the free "Starcraft Anthology" version.

Compilation
-----------
Currently, we only develop under Linux, using rust nightly.

Make sure you have these system libraries installed:

* SDL2
* stormlib

Then run `cargo build --release` from the project root dir to build.

Configuration
-------------

Create a file called `settings.toml` in the project root, with the following contents:

``` toml
scdata_path = "<path to your starcraft installation>"
```

