#!/usr/bin/env node

// Downloads the pre-built relay binary for the current platform.
// Falls back to instructions for building from source if no binary available.

const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");
const https = require("https");

const VERSION = "1.2.0";
const REPO = "Manavarya09/relay";

const PLATFORM_MAP = {
  "darwin-arm64": "relay-macos-arm64",
  "darwin-x64": "relay-macos-x86_64",
  "linux-x64": "relay-linux-x86_64",
  "win32-x64": "relay-windows-x86_64.exe",
};

const platform = `${process.platform}-${process.arch}`;
const asset = PLATFORM_MAP[platform];

if (!asset) {
  console.log(`\n  relay: No pre-built binary for ${platform}.`);
  console.log(`  Build from source: cd core && cargo build --release\n`);
  process.exit(0);
}

const binDir = path.join(__dirname, "..", "bin");
const binPath = path.join(binDir, platform === "win32-x64" ? "relay.exe" : "relay");
const wrapperPath = path.join(binDir, "relay");

// Skip if binary already exists
if (fs.existsSync(binPath)) {
  process.exit(0);
}

fs.mkdirSync(binDir, { recursive: true });

const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${asset}`;

console.log(`  relay: Downloading binary for ${platform}...`);

function download(url, dest, redirects = 0) {
  if (redirects > 5) {
    console.log("  relay: Too many redirects. Build from source instead.");
    writeSourceFallback();
    return;
  }

  https.get(url, { headers: { "User-Agent": "relay-npm" } }, (res) => {
    if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
      download(res.headers.location, dest, redirects + 1);
      return;
    }

    if (res.statusCode !== 200) {
      console.log(`  relay: Download failed (HTTP ${res.statusCode}).`);
      console.log(`  Build from source: cd node_modules/@masyv/relay/core && cargo build --release`);
      writeSourceFallback();
      return;
    }

    const file = fs.createWriteStream(dest);
    res.pipe(file);
    file.on("finish", () => {
      file.close();
      fs.chmodSync(dest, 0o755);
      console.log(`  relay: Installed to ${dest}`);

      // Write wrapper script for non-windows
      if (process.platform !== "win32") {
        fs.writeFileSync(wrapperPath, `#!/bin/sh\nexec "${binPath}" "$@"\n`);
        fs.chmodSync(wrapperPath, 0o755);
      }
    });
  }).on("error", () => {
    console.log("  relay: Download failed. Build from source instead.");
    writeSourceFallback();
  });
}

function writeSourceFallback() {
  fs.writeFileSync(
    wrapperPath,
    `#!/bin/sh\necho "relay binary not found. Build from source:"\necho "  cd node_modules/@masyv/relay/core && cargo build --release"\nexit 1\n`
  );
  fs.chmodSync(wrapperPath, 0o755);
}

download(url, binPath);
