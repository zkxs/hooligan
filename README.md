# Hooligan

Hooligan is a utility that resets manually shown avatars in VRChat before each play session.

## Installation and Usage

1. Download hooligan.exe from the [latest release](https://github.com/zkxs/hooligan/releases/latest) to a location of
   your choice. Make note of where you saved it.
2. In Steam, change the VRChat launch option to `<INSTALL_LOCATION>\hooligan.exe %command%`, where `<INSTALL_LOCATION>`
   is where you saved hooligan.exe. For example, if you saved it to `C:\Users\Tupper\Downloads\` then you should use
   `C:\Users\Tupper\Downloads\hooligan.exe %command%` as the Steam launch option.

Hooligan will now automatically run as you start VRChat.

## FAQ

### Why make Hooligan?

I personally use very aggressive performance rank settings to save my FPS, so find it annoying when I manually show
someone's avatar and then a year later I run into them again in the least optimized avatar I've ever seen. Hooligan
solves this problem by making all manually shown avatars temporary to a single play session.

### What does Hooligan do?

It just edits your 
[LocalPlayerModerations file](https://docs.vrchat.com/docs/local-vrchat-storage#localplayermoderations-file-format) to
remove all shown avatar entries. This file has a .vrcset extension and contains data on players for whom you've manually
shown or hidden their avatar.

### What's up with the weird Steam launch option?

When you pass `%command%` the Steam launch options for a game, it does NOT run the game command. It instead executes the entire launch options as
a command, and the actual game launch command is substituted in where `%command%` is.

So `C:\hooligan.exe %command%` will become `C:\hooligan.exe C:\Steam\steamapps\common\VRChat\launch.exe`, or in desktop mode `C:\hooligan.exe C:\Steam\steamapps\common\VRChat\launch.exe --no-vr`

After Hooligan cleans up your LocalPlayerModerations file it will take those launch options and run them to start VRChat.

Note that you can still use Hooligan without this process launching behavior simply by not passing any arguments to it.

### Why is this called Hooligan?

VRChat Local Player Moderation Manager is too long, and this is used to unshow hooligans' avatars before they change
into something with terrible performance while you're not playing. Also, I like the word "hooligan".

## Installing from Source

1. [Install Rust](https://www.rust-lang.org/tools/install)
2. `cargo install hooligan`

## Building from Source

1. [Install Rust](https://www.rust-lang.org/tools/install)
2. Clone the project
3. `cargo build --release`

## License

Copyright 2024

Hooligan is provided under the [GPL-3.0 license](LICENSE).
