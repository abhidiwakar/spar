# Microsoft Store (MSIX)

Spar’s Store path is **MSIX**. You upload an unsigned `.msix`. After certification, Microsoft signs it. You do not need a code-signing certificate.

This is a different product type from the EXE/MSI listing. Create a new **MSIX** app in Partner Center if the current one is EXE/MSI.

## 1. Create the MSIX product

1. Partner Center → Apps and games → **New product** → **MSIX or PWA app**.
2. Reserve the name (for example `Spar`).
3. Open **Product identity** (or App identity) and copy:
   - **Package/Identity/Name** — looks like `AbhishekDiwakar.Spar` or a Store-generated id
   - **Package/Identity/Publisher** — looks like `CN=XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX`
4. Put those values in `msix/identity.json`:

```json
{
  "identityName": "paste Package/Identity/Name here",
  "displayName": "exact reserved name from Partner Center",
  "publisher": "CN=paste-publisher-id-here",
  "publisherDisplayName": "Diwakar IT Services"
}
```

`displayName` must match a **reserved** app name exactly (including spaces). `Package/Properties/DisplayName` is not the Start-menu nickname. `publisher` must include the `CN=` prefix.

Identity values live in `msix/identity.json` (they are public in every installed package). Pushes to `main` pack an unsigned `.msix` on `windows-latest` and attach it to the GitHub Release.

## 2. Build the package (Windows)

Needs the [Windows SDK](https://developer.microsoft.com/windows/downloads/windows-sdk/) (`makeappx.exe`).

```powershell
npm ci
npm run tauri build
npm run pack:msix
```

Output:

`src-tauri/target/release/bundle/msix/Spar_<version>.0_x64.msix`

On `main`, CI does this after the Tauri Windows build and uploads `spar-msix` as a workflow artifact as well.

## 3. Upload

1. In the **MSIX** product, start a submission.
2. Packages → upload the `.msix`. Leave it **unsigned**.
3. Reuse the listing copy (description, AI disclosure, features) from the earlier Store draft.
4. For capabilities, Partner Center may ask why `runFullTrust` is used. Use:

   Spar is a Win32 desktop app (Tauri). Run and Submit execute the user’s Python or Node.js on this machine. Progress is a local SQLite file. Full trust is required to launch those runtimes and write app data. There is no in-process sandbox.

5. Under device families, leave **Windows 10/11 Desktop** checked. Uncheck Xbox, HoloLens, Surface Hub, Team, and IoT unless you upload packages for those.
6. Submit.

If validation says the display name is not reserved, `displayName` in `msix/identity.json` does not match a name on **Manage app names**. Use that exact string, or reserve `Spar` there if you want the package to keep saying Spar.

## What you do not do

- Do not sign the `.msix` with a test or CA cert before Store upload.
- Do not paste a GitHub Releases or Drive URL. MSIX is a file upload.
- Do not reuse the EXE/MSI package-URL form for this file.

GitHub Releases can still ship the unsigned `.exe` / `.msi` for people who sideload. SmartScreen may warn on those. The Store listing should use only the MSIX.
