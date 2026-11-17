# Homebrew Cask Distribution Guide for RAILbench

This guide explains how **RAILbench** is distributed to macOS users via [Homebrew](https://brew.sh) as a Homebrew Cask.

---

## 1. Overview

RAILbench is a graphical macOS desktop application packaged inside `.dmg` disk images for both Apple Silicon (`arm64`) and Intel (`x64`) architectures. Homebrew distributes GUI applications using **Casks**.

- **Cask File:** `packaging/homebrew/Casks/railbench.rb`
- **Application Bundle:** `railbench.app` (installed to `/Applications/railbench.app`)
- **Bundle ID:** `de.railbench.app`
- **Supported macOS Versions:** macOS 10.15 (Catalina) or newer

---

## 2. Setting Up a Personal Homebrew Tap

The fastest and most flexible way to provide Homebrew installations is to host a **Custom Tap** under your GitHub account (`Nicolaikurz/homebrew-tap`).

### Step 1: Create the GitHub repository
1. Go to [https://github.com/new](https://github.com/new).
2. Name the repository: **`homebrew-tap`** (Homebrew requires this naming convention so `brew tap Nicolaikurz/tap` resolves automatically).
3. Set visibility to **Public**.
4. Initialize with a `README.md`.

### Step 2: Push the Cask definition to your tap
Clone your new tap repo and copy the Cask file:

```bash
git clone https://github.com/Nicolaikurz/homebrew-tap.git
cd homebrew-tap

# Create Casks directory
mkdir -p Casks

# Copy the generated railbench.rb cask
cp /path/to/railbench/packaging/homebrew/Casks/railbench.rb Casks/railbench.rb

# Commit and push
git add Casks/railbench.rb
git commit -m "Add railbench cask v0.6.0"
git push origin main
```

---

## 3. How Users Install RAILbench

Once the tap is published on GitHub, any macOS user with Homebrew installed can install RAILbench with a single command:

```bash
# Option A: Tap once, then install
brew tap Nicolaikurz/tap
brew install --cask railbench

# Option B: Direct one-liner
brew install --cask Nicolaikurz/tap/railbench
```

### Upgrading:
```bash
brew upgrade --cask railbench
```

### Uninstalling:
```bash
brew uninstall --cask railbench
```
*(The cask includes a `zap trash:` block that cleans up application preferences and caches in `~/Library/Application Support/railbench` when run with `--zap`)*

---

## 4. Automated Cask Updates on New Releases

After generating new macOS DMGs for a release:

1. Build the macOS DMGs:
   ```bash
   npm run dist:mac:dmg
   ```
2. Run the Homebrew updater script:
   ```bash
   npm run dist:homebrew
   # or: ./scripts/update-homebrew-cask.sh
   ```
   This automatically reads the version from `package.json`, calculates the SHA-256 hashes of `railbench-<version>-mac-arm64.dmg` and `railbench-<version>-mac-x64.dmg`, and updates `packaging/homebrew/Casks/railbench.rb`.
3. Copy the updated `packaging/homebrew/Casks/railbench.rb` to your `homebrew-tap` repository and push.

---

## 5. Submitting to Official Homebrew (`homebrew/cask`)

Once RAILbench reaches sufficient GitHub stars and public release history, you can submit the cask directly to the official Homebrew repository:

1. Fork [https://github.com/Homebrew/homebrew-cask](https://github.com/Homebrew/homebrew-cask).
2. Test the cask with Homebrew audit tools:
   ```bash
   brew audit --cask --new Casks/railbench.rb
   brew style --cask Casks/railbench.rb
   ```
3. Commit and open a Pull Request against `Homebrew/homebrew-cask`.
4. Once merged, users can install RAILbench simply by typing:
   ```bash
   brew install --cask railbench
   ```

---

## 6. macOS Gatekeeper Note

If users encounter a `"railbench.app cannot be opened because the developer cannot be verified"` notice upon first launch (for ad-hoc signed releases without an Apple Developer ID certificate):

Users can allow execution with:
```bash
xattr -cr /Applications/railbench.app
```
Or right-click `railbench.app` in Finder and select **Open**.
