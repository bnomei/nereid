#!/usr/bin/env node
// npm distribution shim: downloads the platform release binary into a local cache and forwards argv.
"use strict";

const childProcess = require("child_process");
const crypto = require("crypto");
const fs = require("fs");
const https = require("https");
const os = require("os");
const path = require("path");

const packageJson = require("../package.json");

const BIN = "nereid";
const REPOSITORY = process.env.NEREID_REPOSITORY || "bnomei/nereid";
const VERSION = normalizeVersion(process.env.NEREID_VERSION || packageJson.version);

main().catch((error) => {
  console.error(`nereid npm wrapper: ${error.message}`);
  process.exit(1);
});

async function main() {
  const release = releaseTarget();
  const binaryPath = path.join(cacheDir(), VERSION, release.target, release.binary);

  if (!fs.existsSync(binaryPath)) {
    await installRelease(binaryPath, release);
  }

  const result = childProcess.spawnSync(binaryPath, process.argv.slice(2), {
    stdio: "inherit",
  });

  if (result.error) {
    throw result.error;
  }

  if (result.signal) {
    process.kill(process.pid, result.signal);
    return;
  }

  process.exit(result.status || 0);
}

function normalizeVersion(version) {
  return version.startsWith("v") ? version : `v${version}`;
}

function releaseTarget() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === "linux" && arch === "x64") {
    return unixRelease("x86_64-unknown-linux-musl");
  }
  if (platform === "linux" && arch === "arm64") {
    return unixRelease("aarch64-unknown-linux-musl");
  }
  if (platform === "darwin" && arch === "x64") {
    return unixRelease("x86_64-apple-darwin");
  }
  if (platform === "darwin" && arch === "arm64") {
    return unixRelease("aarch64-apple-darwin");
  }
  if (platform === "win32" && arch === "x64") {
    return {
      target: "x86_64-pc-windows-msvc",
      archiveExt: ".zip",
      binary: `${BIN}.exe`,
    };
  }

  throw new Error(`unsupported platform ${platform}/${arch}`);
}

function unixRelease(target) {
  return {
    target,
    archiveExt: ".tar.gz",
    binary: BIN,
  };
}

function cacheDir() {
  if (process.env.NEREID_NPM_CACHE) {
    return process.env.NEREID_NPM_CACHE;
  }

  if (process.platform === "win32") {
    return path.join(process.env.LOCALAPPDATA || os.tmpdir(), "nereid", "npm");
  }

  return path.join(
    process.env.XDG_CACHE_HOME || path.join(os.homedir(), ".cache"),
    "nereid",
    "npm",
  );
}

async function installRelease(binaryPath, release) {
  const archive = `${BIN}-${VERSION}-${release.target}${release.archiveExt}`;
  const baseUrl = `https://github.com/${REPOSITORY}/releases/download/${VERSION}/${archive}`;
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "nereid-npm-"));

  try {
    const archivePath = path.join(tmp, archive);
    const checksumPath = `${archivePath}.sha256`;

    await download(`${baseUrl}.sha256`, checksumPath);
    await download(baseUrl, archivePath);
    verifyChecksum(archivePath, checksumPath);
    extractArchive(archivePath, tmp, release.archiveExt);

    const extracted = path.join(tmp, release.binary);
    if (!fs.existsSync(extracted)) {
      throw new Error(`release archive did not contain ${release.binary}`);
    }

    fs.mkdirSync(path.dirname(binaryPath), { recursive: true });
    fs.copyFileSync(extracted, binaryPath);
    if (process.platform !== "win32") {
      fs.chmodSync(binaryPath, 0o755);
    }
  } finally {
    fs.rmSync(tmp, { force: true, recursive: true });
  }
}

function download(url, destination, redirects = 0) {
  return new Promise((resolve, reject) => {
    const request = https.get(
      url,
      {
        headers: {
          "user-agent": "nereid-npm-wrapper",
        },
      },
      (response) => {
        if (
          response.statusCode >= 300 &&
          response.statusCode < 400 &&
          response.headers.location
        ) {
          response.resume();
          if (redirects > 5) {
            reject(new Error(`too many redirects downloading ${url}`));
            return;
          }
          download(response.headers.location, destination, redirects + 1).then(resolve, reject);
          return;
        }

        if (response.statusCode !== 200) {
          response.resume();
          reject(new Error(`download failed with HTTP ${response.statusCode}: ${url}`));
          return;
        }

        const file = fs.createWriteStream(destination);
        response.pipe(file);
        file.on("finish", () => file.close(resolve));
        file.on("error", reject);
      },
    );

    request.on("error", reject);
  });
}

function verifyChecksum(archivePath, checksumPath) {
  const expected = fs.readFileSync(checksumPath, "utf8").trim().split(/\s+/)[0].toLowerCase();
  if (!/^[a-f0-9]{64}$/.test(expected)) {
    throw new Error("checksum file did not contain a SHA-256 digest");
  }

  const actual = crypto.createHash("sha256").update(fs.readFileSync(archivePath)).digest("hex");
  if (actual !== expected) {
    throw new Error("checksum mismatch");
  }
}

function extractArchive(archivePath, destination, archiveExt) {
  if (archiveExt === ".tar.gz") {
    childProcess.execFileSync("tar", ["-xzf", archivePath, "-C", destination], {
      stdio: "ignore",
    });
    return;
  }

  const powershell = path.join(
    process.env.SystemRoot || "C:\\Windows",
    "System32",
    "WindowsPowerShell",
    "v1.0",
    "powershell.exe",
  );
  childProcess.execFileSync(
    powershell,
    [
      "-NoProfile",
      "-NonInteractive",
      "-ExecutionPolicy",
      "Bypass",
      "-Command",
      "Expand-Archive -LiteralPath $args[0] -DestinationPath $args[1] -Force",
      archivePath,
      destination,
    ],
    { stdio: "ignore" },
  );
}
