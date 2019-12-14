# randomprime

This is a fork of syncathetic's [randomprime](https://github.com/aprilwade/randomprime) program, and acts as a backend for [Metroid Prime Door Randomizer](https://github.com/YonicDev/mpdr) (also known as MPDR).

> **_NOTE:_ It is heavily recommended you use MPDR which is the GUI frontend and also generates profiles. You can download it at the link above.**

It does _not_ randomize the pickup layout, and needs a separate profile to work, instead of a layout descriptor.

## How to use the ISO patcher

If you're on Windows, you can launch the patcher by simply double clicking the EXE file in Explorer.
Alternatively, you can drag-and-drop your input ISO onto the EXE file to avoid manually typing its location later.

The patcher can also be run from a terminal.
If you run it without passing any arguments, it'll operate in interactive mode, just like when its launched from the GUI.
The patcher also has a CLI, the details of which you can find by running it with the `-h` flag.

## Reporting a bug

If you file an issue, please include the profile you used, a hash of the input ISO, and a hash of the generated ISO.

## Q & A

##### Q: Which versions of Metroid Prime are supported?
A:
Only the NTSC 0-00 and 0-02 (aka 1.00 and 1.02) versions are supported.
The 00-1 NTSC version, non-NTSC versions and the trilogy version will not work.
Hashes of a known good 0-00 ISO dump are:
```
MD5:  eeacd0ced8e2bae491eca14f141a4b7c
SHA1: ac20c744db18fdf0339f37945e880708fd317231
```

##### Q: Can a patched ISO be used as the input ISO?
A:
No, you must use a clean/unpatched input ISO.

##### Q: Will you merge the item randomizer and door randomizer?
A:
I might consider it in the future.

##### Q: Are all seeds clearable?
A:
They _should_ as long as the weights for non-blue doors are small enough.

##### Q: Will you support Metroid Prime 2: Echoes?
A: No, because it would be trivial, as the Dark Beam and Light Beam are obtained very early in the game, and the Annihilator Beam is acquired near the end of it.

## Thanks

The creation of this tool would not have been possible without the Metroid Prime Modding community in Discord, especially [syncathetic](https://github.com/aprilwade), the original creator of randomprime.
