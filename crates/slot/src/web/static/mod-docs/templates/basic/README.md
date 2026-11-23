# Minimal linkSlot mod

This template is intentionally small. Rename the directory to a valid mod id
(`mods/<id>/`), then enable it in the device Web mods page.

- `mod.json`: schema 2 manifest; `permissions` is for the Web bridge.
- `actions/demo/action.json`: manual action with the cloud-sync card.
- `actions/demo/main.lua`: runs when the user picks the card and presses A.

Read the bundled `API.md` before adding file or network work. Do not use
`os`, `io`, `require`, `package`, `debug`, or unlisted globals.
