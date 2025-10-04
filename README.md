# Minecraft Mod Syncer

A simple tool for downloading necessary mods (and/or optionals) from [my mod hosting webserver project](https://github.com/almafa64/minecraft-mod-hoster) and deletes unneeded mods from local (with ability to flag mods as don't delete).

## Usage
1. Download executable from `Releases` (or build it from source) then run it.
1. Input the host webserver's address and path to your mods folder.
1. Change to the branch you want to use.
1. Optional steps:
    - Select optional mods to download from `to download` list.
    - Select mods to not delete from `to delete` list.
1. Press `Download`

## Todos
- [X] download zip + unzip it
- [X] download files separately
- [X] delete mods from local
- [X] show optional mods in delete list so they can be deleted
- [ ] checksum comparison
- [ ] settings menu
  - [ ] colors
  - [ ] themes?
  - [ ] text aligment
  - [ ] config file path
  - [ ] translation
- [ ] grey out required mods
- [ ] grey out optional to deletes
- [X] optional mods
- [ ] checking version changes (must download/delete)
- [X] saving uncheked to_delete mods to keep file + load them
- [X] profiles
- [X] about dialog
- [X] check new version from github
- [ ] auto update from github
- [ ] CLI

## Known bugs
If the host server gets a new branch while this program runs it won't fetch it.

## Building
### Prerequisits
- cargo
- rustup
- gcc
- make
- cmake
- linux specifics:
  - Xinerma
  - pkg-config
- windows specifics:
  - dlltool (mingw)

### Steps
1. run `cargo build --release`
1. if successfully exited, executable will be at `target/release/minecraft-mod-syncer(.exe)`

## FAQ
### For who is this tool?
- Players on custom modded servers. (Especially on servers where mods change)
- People who play from multiple machines (e.g. home pc, laptop, school pc, etc.).

### Why use this rather than
- **downloading mods manually?**<br>
It's easy to download the wrong version of a mod or forget one, which can block you from joining the server. This tool won't let that happen.
- **downloading premade modpack?**<br>
If a server uses a premade modpack (like ATM10), then yes, this program won't help. This tool is for custom modded servers.
- **share mods/zips with each other?**<br>
This tool does that, but better, faster and it's way more easier for everyone.

### Why did I make this?
**Warning**: yapping

I hosted minecraft servers for my friends with tons of mods, and I had enough of constantly needing to share zips with them.<br>
So I made a website for them, thus they can easily download the dynamically built zips (from 2 folder /both, /client_only (and /server_only which isn't zipped) this way I could build the zips and clone mods into the server automaticly).<br>
But still, they were too lazy to use that page, therefor I made a python script for them (which was the cli predecessor of this program). But I felt this program could be used by more people who change mods in the middle of hosting a server, so I upgraded it to a GUI app and expanded its features.

Another big reason for doing all this, was to learn rust, gui, cross compiling, async/threading and web requests in rust. And oh boy did I learn a lot from this. This was my 3rd rust project (and the previous 2 was nowhere this size). 1.0 took 4 months (with other side projects in that time, so actually it was more like 1 month of work).
