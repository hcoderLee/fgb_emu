# fgb_emu

A plugin implement Gameboy color emulator.

## How to build

If you want to modify the emulator's fuctions or add new features, you need to know how to build the
project.

### Build rust code

The core of Gameboy emulator is implement by Rust, and the source code is located at `native/gb_emu`
, we need to build it first.

We have to setup rust environment before build, the first thing is
to [Install rust](https://www.rust-lang.org/tools/install).

Then we need add platform target for
Android and iOS, run the command below:

```shell
rustup target add \
  armv7-linux-androideabi \
  aarch64-linux-android \
  aarch64-apple-ios
```

**Target** means stander library for that platform. We only install
`ARM` target for Android (`armv7-linux-androideabi` for 32 bit phone, `aarch64-linux-android` for 64
bit phone) here, you may need to add `X86` target if running on Android emulator, for more detail, please
check [Cross-compilation](https://rust-lang.github.io/rustup/cross-compilation.html)

In Android, we will use **cargo ndk** to build **.so** dynamic library. Run cammand below to install it

```shell
cargo install cargo-ndk
```

Change the current directory to `./native`, and run

```shell
cargo ndk -t arm64-v8a -o target/aarch64-linux-android/release build --release
```

### Integrate native lib

When you have build the rust code, you can find the dynamic lib in `native/gb_emu/target/aarch64-linux-android/release/libgb_emu.so`. We need integrate it into android project. Move it to `./android/src/main/jniLibs/arm64-v8a` directory. Now the flutter project can find and load this lib for Android. Run command below in `./native/gb_emu`:

```shell
cp target/aarch64-linux-android/release/libgb_emu.so ../../android/src/main/jniLibs/arm64-v8a/
```

### Generate FFI file

Finally you need to call functions in the lib build from Rust code, this technique is called **FFI**. You can learn it from: [C interop using dart:ffi](https://dart.dev/guides/libraries/c-interop).

We use [ffigen](https://pub.dev/packages/ffigen) to generate **ffi** related code. It need a header file which include all of functions provided by rust code. You can find it in `./native/includes/gb_emu.h`, When you change or add new functions in rust code, remember to change this file too if needed

Change the curren directory to project root directory, and run the command below to generate ffi related dart code (which in `lib/src/native/gb_native_binding.dart`)

```shell
flutter pub run ffigen
```

## How to use

You can use this plugin to implement you own Gameboy emulator, the only thing you need to do is specific a game rom file (suffix with .gb or .gbc) path. (You can download from [EmulatorGames.net](https://www.emulatorgames.net/roms/gameboy/)).

The example of this plugin demonstrate how to use it to implement a simple Gameboy emulator
