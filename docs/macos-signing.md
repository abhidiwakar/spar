# macOS signing and notarization

CI signs and notarizes the Apple Silicon `.dmg` with a **Developer ID Application** certificate. Without these GitHub values the macOS package job fails.

Add them under **Settings → Secrets and variables → Actions**. Never commit the `.p12` or `.p8`.

## Certificate

1. On a Mac, create a [Certificate Signing Request](https://developer.apple.com/help/account/create-certificates/create-a-certificate-signing-request).
2. In [Certificates, Identifiers & Profiles](https://developer.apple.com/account/resources/certificates/list) create a **Developer ID Application** certificate (not Apple Development, not Apple Distribution).
3. Download the `.cer`, open it to install it in Keychain Access.
4. In Keychain Access → login → My Certificates, expand the cert, right-click the private key, **Export**, save a `.p12`, and set a password.

```bash
openssl base64 -A -in /path/to/certificate.p12 -out certificate-base64.txt
```

5. `security find-identity -v -p codesigning` — you should see `Developer ID Application: Your Name (TEAMID)`.

## App-specific password

At [appleid.apple.com](https://appleid.apple.com) → Sign-In and Security → App-Specific Passwords, create one for `Spar CI`. This is **not** your Apple ID password.

Your Team ID is on the [membership page](https://developer.apple.com/account#MembershipDetailsCard).

## GitHub Actions

**Secrets**

| Name | Value |
| --- | --- |
| `APPLE_CERTIFICATE` | Entire contents of `certificate-base64.txt` (one line) |
| `APPLE_CERTIFICATE_PASSWORD` | Password you set when exporting the `.p12` |
| `APPLE_PASSWORD` | App-specific password |

**Variables**

| Name | Value |
| --- | --- |
| `APPLE_ID` | Developer Apple ID email |
| `APPLE_TEAM_ID` | 10-character team ID |

The signing identity is read from the imported certificate. Do not use an Apple Development cert — Gatekeeper will still reject the app.

## After the next `main` push

The release `.dmg` should open without “Spar is damaged”. First notarization can take several minutes. If the job hangs on notarization, accept any new Apple Developer agreements in the account portal.
