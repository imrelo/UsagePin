# UsagePin

A Windows desktop usage indicator built with Tauri, React, and TypeScript.

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
