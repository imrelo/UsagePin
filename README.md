# UsagePin

A Windows desktop usage indicator built with Tauri, React, and TypeScript.

<<<<<<< HEAD
Choose **Codex** or **Antigravity** in **Settings → Account connection → Widget displays**. The widget shows only the selected service, and remembers the choice across restarts. Reset restores Codex as the default.

## Antigravity connection

Keep the Antigravity Windows desktop app running and signed in. UsagePin discovers its local HTTP port from `%APPDATA%\Antigravity\logs\language_server.log` and reads `RetrieveUserQuotaSummary` through the app's existing local session. It displays Gemini and Claude/GPT weekly remaining quotas and reset dates. The connection check and configured refresh interval apply to Antigravity as well.

UsagePin reads the app's quota summary with `forceRefresh=false`, following the Models & Usage UI. This is not an independent direct request to Google's quota API. If the app is closed or the connection fails, quota is cleared and the widget shows the connection state. Local session tokens stay in memory in the Rust backend and are sent only to `127.0.0.1`, with proxies and redirects disabled. No OmniRoute credentials are read, and no model or OAuth token-refresh requests are made by UsagePin. This integration currently supports the standalone desktop app; older Antigravity IDE versions may use a different API.

=======
>>>>>>> b2c37d6f8050781195e3aa58c50efdd5b8c894fc
## Codex account connection

Sign in to the Codex Windows app with ChatGPT, then open UsagePin's **Settings → Account connection** and select **Check connection / Refresh**. UsagePin reads the existing session from `CODEX_HOME/auth.json`, or `%USERPROFILE%\.codex\auth.json` by default, and fetches the remaining quota and reset times directly from Codex. The session file is read only; access tokens stay in the Rust backend.

The default refresh interval is 60 seconds and can be changed in Settings. On connection errors, previously fetched quota is marked as stale. If the session expires, open Codex to update the session or sign in again, then check the connection. API-key sessions and sessions stored only in Windows Credential Manager are not supported in this version.

## Development setup

Install Node.js, Rust (MSVC toolchain), Microsoft Visual C++ Build Tools, and the Windows SDK. Then run:

```powershell
npm ci
npm run tauri dev
```

## Build

```powershell
npm run build
npm run tauri build
```

## Debug in VS Code

Install the recommended CodeLLDB extension. In Run and Debug, select **UsagePin: Frontend + Backend**, then press **F5**. This starts Vite, builds the Rust backend, and attaches both the Rust and WebView2 debuggers. Breakpoints work in `frontend/` and `backend/src/`; **Shift+F5** stops both debug sessions.

Debug builds use `backend/target-debug` inside the repository.

## Push to GitHub

Create an empty GitHub repository named `UsagePin`, then set a project-local commit identity and push the `main` branch:

```powershell
git config --local user.name "Your Name"
git config --local user.email "YOUR_GITHUB_NOREPLY_EMAIL"
git remote add origin https://github.com/YOUR_USERNAME/UsagePin.git
git add .
git commit -m "Initial project setup"
git push -u origin main
```

Use the email shown in your GitHub email settings, or your GitHub-provided `noreply` address. These commands configure this repository only.

See [PUBLISHING.md](PUBLISHING.md) for the Microsoft Store packaging path and the remaining account-specific steps.
