# Isle 0.1.1 — Windows 11 x64

Run `Isle.exe` for the portable app, or run `Isle_0.1.1_x64-setup.exe` to install.
WebView2 is required. Click the taskbar pill to open Shelf and the other tools.
Use the tray menu if the pill's position needs changing.

Enable clipboard history explicitly in History or Settings. Choose an existing
signed-in Chrome profile in Settings before using Gmail, Outlook or WhatsApp.

The release build and 24 automated tests pass. Native interaction acceptance,
fresh-VM installation and one-hour performance targets remain unverified; see
`../MILESTONES.md`. Codex usage and grammar checking are deferred.

This build corrects text showing through the panel, stretched stars, overlapping
download/empty content and tab changes clearing pinning or jumping to Shelf.
Native file-drop handling now covers WebView child windows as well as the outer
window. The user's PDF retry succeeded and native diagnostics confirmed one file
added to Shelf. Other formats and drag-out destinations need separate verification.

SHA-256:

```text
Isle.exe
F303315B80EC37360268D2E3A2C48EF695001DD7788418153CB04A2F86F5CA91
Isle_0.1.1_x64-setup.exe
B9915B58094AE8D5B6510DA4F3515CC02E08248C7B0052CAD2A22839AD0EF922
```
