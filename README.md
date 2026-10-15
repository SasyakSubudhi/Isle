# Isle

A Windows 11 taskbar island. Implementation proceeds through the gated milestones
in [PLAN.md](PLAN.md); current evidence is in [MILESTONES.md](MILESTONES.md).

## Development

Install Node.js, Rust's `x86_64-pc-windows-msvc` toolchain, Visual Studio C++ build
tools, a Windows SDK and WebView2. Then run:

```powershell
npm ci
npm run tauri dev
```

The Tauri launcher also finds Rust in rustup's standard user directory, so a
terminal restart after installation is not required.

The current M0 scaffold renders a static pill. Other modules are not implemented
yet. It is not a complete release.
