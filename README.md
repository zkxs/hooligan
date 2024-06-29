# Hooligan

Hooligan is a utility that manages VRChat's
[LocalPlayerModerations files](https://docs.vrchat.com/docs/local-vrchat-storage#localplayermoderations-file-format).
These files have a .vrcset extension and contain data on players for whom you've manually shown or hidden their avatar.

# Features

- Automatically clear shown avatars
- Users can be whitelisted to exempt them from the auto-clear
- Tracks the number of times you've shown someone's avatar
- Users who have been shown a certain number of times can be exempted from the auto-clear

# TODO

- Automatically runs before starting VRChat
- Config management UI

# Usage

<!-- TODO -->

# FAQ

## Why is this called Hooligan?

VRChat Local Player Moderation Manager is too long, and this is used to unshow hooligans' avatars before they change
into something with terrible performance while you're not playing. Also, I like the word "hooligan".

## Why does my antivirus hate this?

- I'm not paying upwards of 50 USD per year for a code signing cert, and antivirus software dislikes unsigned code.
- I'm _definitely_ not paying upwards of 250 USD per year for a more trusted EV (extended validation) code signing cert.
- Some antivirus software conflates a lack of C-style struct and function definitions with obfuscation. Spoiler alert:
  this application isn't written in C.
- It's not my job to fix antivirus false positives. I'm not going to spend my time begging various antivirus vendors to
  fix their shit.

## What's up with the Steam launch option thing?

When you pass `%command%` the Steam launch options for a game, it does NOT run the game command. It instead executes the entire launch options as
a command, and the actual game launch command is substituted in where `%command%` is. 

So `C:\hooligan.exe %command%` will become `C:\hooligan.exe C:\Steam\steamapps\common\VRChat\launch.exe`, or in desktop mode `C:\hooligan.exe C:\Steam\steamapps\common\VRChat\launch.exe --no-vr`

After Hooligan cleans up your LocalPlayerModerations file it will take those launch options and run them to start VRChat.
