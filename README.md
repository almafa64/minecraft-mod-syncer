# Minecraft Mod Syncer

A simple tool for downloading necessary mods from [my custom host webserver](https://github.com/almafa64/minecraft-mod-hoster) and deletes unneeded mods from local.

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
- **instances?**<br>
This tool can be used on top of instances (I do the same), it's just for syncing mods with server.

### Why did I make this?
**Warning**: tons of yapping

I hosted minecraft servers for my friends with ton of mods, and I had enough of constantly needing to share zips with them.<br>
So I made a website for them, thus they can easily download the dynamically built zips (from 2 folder /both, /client_only (and /server_only which isn't zipped) this way I could build the zips and clone mods into the server automaticly).<br>
But still, they were too lazy to use that page, therefor I made a python script for them (which was the cli predecessor of this program).

Another big reason for doing all this, was to learn rust, gui, cross compiling, async/threading and web requests in rust. And oh boy did I learn a lot from this. This was my 3rd rust project (and the previous 2 was nowhere this size).

