# fgb_emu

A plugin implement Gameboy color emulator.

## How to build

If you want to modify the emulator's fuctions or add new features, you need to know how to build the
project.

### Build rust code

The core of Gameboy emulator is implement by Rust, and the source code is located at `native/gb_emu`
, to setup rust environment before build, [install rust](https://www.rust-lang.org/tools/install)
first.

Then install [cargo-make](https://github.com/sagiegurari/cargo-make), it's a tool to config build
procedure for android and iOS native lib

```shell
cargo install --force cargo-make
```

Run follow command to build Android and iOS native lib (Only build iOS lib on MacOS)

```shell
cargo make build
```

After running this task, it will generate `arm-v7a` and `arm-v8a` **.so** lib for Android, which
located in `android/src/main/jniLibs`

You can running `cargo make clean` to clear build caches, the whole rust project will be rebuild
when you execute build cammand next time

### Generate FFI file

Finally you need to call functions in the lib build from Rust code, this technique is called **FFI**
. You can learn it from: [C interop using dart:ffi](https://dart.dev/guides/libraries/c-interop).

We use [ffigen](https://pub.dev/packages/ffigen) to generate **ffi** related code. It need a header
file which include all of functions provided by rust code. You can find it
in `./native/includes/gb_emu.h`, When you change or add new functions in rust code, remember to
change this file too if needed

Change the curren directory to project root directory, and run the command below to generate ffi
related dart code (which in `lib/src/native/gb_native_binding.dart`)

```shell
flutter pub run ffigen
```

## How to use

You can use this plugin to implement you own Gameboy emulator, the only thing you need to do is
specific a game rom file (suffix with .gb or .gbc) path. (You can download
from [EmulatorGames.net](https://www.emulatorgames.net/roms/gameboy/)).

The example of this plugin demonstrate how to use it to implement a simple Gameboy emulator
