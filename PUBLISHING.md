# Microsoft Store preparation

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
npm run store:windows
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
