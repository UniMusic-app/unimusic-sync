# UniMusic Sync

A library to allow easy synchronisation between devices for [UniMusic](https://github.com/UniMusic-app/unimusic).

## Development

This repository is based on [ianthetechie's UniFFI Starter](https://github.com/ianthetechie/uniffi-starter/), which streamlined the process A LOT. Much appreciated.

### Rust

Open up the project in your favorite editor and poke around the Cargo workspace
under `rust/`!

### iOS

Before opening up the Swift package in Xcode, you need to build the Rust core.

```shell
cd rust/
./build-ios.sh
```

This generates an XCFramework and generates Swift bindings to the Rust core.
Check the script if you're interested in the gritty details.

> [!IMPORTANT]
> You need to do this every time you make Rust changes that you want reflected in the Swift Package!

### Android

Android is pretty easy to get rolling, and Gradle will build everything for you
after you get a few things set up.
Most importantly, you need to install [`cargo-ndk`](https://github.com/bbqsrc/cargo-ndk).

```shell
cargo install cargo-ndk
```

If you've tried building the Rust library already and you have rustup,
the requisite targets will probably be installed automatically.
If not, follow the steps in the [`cargo-ndk` README](https://github.com/bbqsrc/cargo-ndk)
to install the required Android targets.

Just open up the `android` project in Android Studio and you're good to go.

> [!NOTE]
> If you're having a problem with Android Studio not finding cargo on macOS
> Try opening Android Studio within `android` directory using `open -na "Android Studio.app"`
