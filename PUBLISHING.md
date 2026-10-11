# Microsoft Store release: MSIX x86 + x64

The current release uses the **MSIX or PWA app** product reserved in Partner Center. The bundle contains native x86 and x64 packages; Windows selects the matching package and WebView2 runtime.

## Product identity

- Package name: `Relo.UsagePin`
- Publisher: `CN=30F1A696-67CD-45AB-9E89-3750813AB217`
- Publisher display name: `Relo`
- First package version: `0.1.0.0`
- Minimum OS: Windows 10 version 2004, build 19041; Windows 11 is also supported.

The manifest is `packaging/msix/AppxManifest.xml`. It declares `runFullTrust` for native windows, the system tray and read-only access to the user's Codex session file. If Partner Center asks for a capability justification, describe those desktop functions. Tokens stay in the Rust backend.

## Build the Store bundle

Install Node.js, Rust MSVC, Visual C++ Build Tools for x86/x64 and the Windows SDK with MakeAppx and SignTool. Then run:

```powershell
rustup target add i686-pc-windows-msvc
npm ci
npm run store:windows
```

`scripts/build-msix.ps1` downloads pinned WebView2 Fixed Runtime packages from Microsoft and verifies their signatures, builds each architecture with a static C runtime and locked Rust dependencies, and packs/bundles the release with MakeAppx semantic validation enabled. Only the executable, matching runtime, manifest and required icons enter each package. Downloaded runtimes and build output are ignored by Git.

Upload this file in **Start submission → Packages**:

```text
backend/target/release/bundle/msix/UsagePin_0.1.0.0_x64_x86.msixbundle
```

Per-architecture MSIX files, MakeAppx logs and SHA256 checksums are in the same directory. To repack existing release builds, run `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-msix.ps1 -SkipBuild`.

The bundle is unsigned for Store submission; Microsoft signs MSIX packages for Store distribution. Installing the upload bundle locally requires a trusted test signature or a supported development deployment workflow. MakeAppx validation does not replace Windows App Certification Kit or installed-app testing.

Complete the Store listing, screenshots, support details and privacy disclosures/policy appropriate to reading an existing Codex session. The package uses the current repository icons. Before final submission, test launch, Settings/tray, resizing, refresh, upgrade and uninstall. Verify the x86 package on a clean Windows 10 32-bit machine; running it on an x64 machine is not equivalent.

For later releases, keep the four-part manifest version aligned with the app version, with the Store revision component set to `0`. Refresh the pinned runtime version and official links in the script together. Windows 7 is outside this release's scope.

References: [Microsoft MSIX components](https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-manual-conversion), [MakeAppx](https://learn.microsoft.com/en-us/windows/msix/package/create-app-package-with-makeappx-tool), [bundling](https://learn.microsoft.com/en-us/windows/msix/package/bundling-overview), [Store submission/signing](https://learn.microsoft.com/en-us/windows/apps/publish/get-started), [WebView2 distribution](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution).

# Legacy NSIS EXE preparation

This separate path is preserved for reference and is not the selected MSIX submission path.

## Packaging path

The current Tauri v2 Store guide uses an unpackaged Win32 installer listing: Tauri produces EXE/MSI installers, and Partner Center registers the product as an EXE or MSI app. Microsoft also supports MSIX and recommends it for packaged apps, but Tauri does not currently generate an MSIX package directly. This project prepares the Tauri-supported NSIS EXE path.

## Store build

`backend/tauri.microsoftstore.conf.json` applies the Store-specific configuration:

- NSIS installer only.
- Offline WebView2 runtime installer, so setup does not need to download WebView2 during installation.
- A publisher placeholder that must be replaced with your actual publisher name before the final build.

Build with:

```powershell
npm ci
npm run store:windows:exe
```

The installer is generated under `backend/target/release/bundle/nsis/`.

Tauri's NSIS installer supports silent installation with `/S`. Enter `/S` as the silent install argument in Partner Center.

## Before submitting

1. Enroll in the Microsoft Store developer program, reserve the product name, and use the Partner Center product details to set the real publisher name in `backend/tauri.microsoftstore.conf.json`.
2. Replace the starter Tauri icons with final UsagePin artwork and regenerate the icon set with `npm run tauri icon <source.png>`.
3. Implement and verify an in-app update path. Tauri's direct EXE/MSI Store route requires the app to handle its own updates.
4. Sign the final EXE with a valid Authenticode certificate. The Store does not re-sign EXE/MSI installers.
5. Host the versioned installer at a stable HTTPS URL. Keep that exact file unchanged after submitting it.
6. Prepare the Store description, support/contact details, privacy disclosures/policy as applicable, and at least one desktop screenshot (PNG, 1366 x 768 pixels or larger).
7. Run the Windows App Certification Kit against the final installer/package and resolve its findings before submission.

The Partner Center identity, publisher name, signing certificate, hosted installer URL, final Store artwork, and store listing text depend on your account and product details and are intentionally left for manual completion.

## References

- [Tauri Microsoft Store distribution](https://v2.tauri.app/distribute/microsoft-store/)
- [Microsoft Store app package requirements](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/app-package-requirements)
- [Microsoft Store desktop app distribution](https://learn.microsoft.com/en-us/windows/apps/distribute-through-store/how-to-distribute-your-win32-app-through-microsoft-store)
- [Store screenshots and images](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/screenshots-and-images)
