# Hooligan

Hooligan is a program that resets manually shown avatars in VRChat between each play session.

If Hooligan notices you've manually shown the same person 3 or more times, then it will "stick" and Hooligan will stop
resetting the shown status for that person. You can always un-show that person again in-game to cause them to unstick.

## Installation and Usage

1. Download [hooligan.exe](https://github.com/zkxs/hooligan/releases/latest/download/hooligan.exe) from the 
  [latest release](https://github.com/zkxs/hooligan/releases/latest) to a location of
   your choice. Make note of where you saved it.
2. In Steam, change the VRChat launch option to `<INSTALL_LOCATION>\hooligan.exe %command%`, where `<INSTALL_LOCATION>`
   is where you saved hooligan.exe. For example, if you saved it to `C:\Users\Tupper\Downloads\` then you should use
   `C:\Users\Tupper\Downloads\hooligan.exe %command%` as the Steam launch option.

Hooligan will now automatically run when you start VRChat from Steam.

## FAQ

### Why make Hooligan?

I personally use very aggressive performance rank settings to save my FPS, so find it annoying when I manually show
someone's avatar and then a year later I run into them again in the least optimized avatar I've ever seen. Hooligan
solves this problem by making all manually shown avatars temporary to a single play session.

### How does Hooligan work?

It just edits your 
[LocalPlayerModerations file](https://docs.vrchat.com/docs/local-vrchat-storage#localplayermoderations-file-format) to
remove shown avatar entries. This file is where VRChat records which players you've manually shown or hidden.

### Where does Hooligan store data?

Logs, configs, and history database are stored in `%localappdata%\hooligan`.

### Can I change the shown avatar sticking threshold?

Yes, open up `%localappdata%\hooligan\config\config.props` and change the `auto_hide_threshold` value there. Anyone
you've manually shown at least that number of times will no longer be automatically un-shown.

### What's up with the weird Steam launch option?

When you put `%command%` in the Steam launch options for a game, Steam will execute the entire launch option text as a
command, and the actual game launch command is substituted in place of `%command%`.

So `C:\hooligan.exe %command%` will become `C:\hooligan.exe C:\Steam\steamapps\common\VRChat\launch.exe`, or in desktop
mode `C:\hooligan.exe C:\Steam\steamapps\common\VRChat\launch.exe --no-vr`

After Hooligan cleans up your LocalPlayerModerations file it will take those launch options and run them to start VRChat.

Note that you can still use Hooligan without this process launching behavior: just don't pass any arguments to it.

### Why Windows?

VRChat only supports Windows natively, so there is not a compelling reason to provide Linux binaries for Hooligan. I
suggest that if you're running VRChat under Proton then you should also run Hooligan within the same Proton prefix.

If you have a use-case for native linux support, please let me know!

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

Copyright 2025

Hooligan is provided under the [GPL-3.0 license](LICENSE).
