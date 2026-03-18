"use strict";
var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/scripts/postinstall.ts
var import_debug = __toESM(require("@prisma/debug"));
var import_engines_version = require("@prisma/engines-version");
var import_fetch_engine = require("@prisma/fetch-engine");
var import_fs = __toESM(require("fs"));
var import_path = __toESM(require("path"));
var debug = (0, import_debug.default)("prisma:download");
var baseDir = import_path.default.join(__dirname, "../../");
var lockFile = import_path.default.join(baseDir, "download-lock");
var createdLockFile = false;
async function main() {
  if (import_fs.default.existsSync(lockFile) && parseInt(import_fs.default.readFileSync(lockFile, "utf-8"), 10) > Date.now() - 2e4) {
    debug(`Lock file already exists, so we're skipping the download of the prisma binaries`);
  } else {
    createLockFile();
    let binaryTargets;
    if (process.env.PRISMA_CLI_BINARY_TARGETS) {
      binaryTargets = process.env.PRISMA_CLI_BINARY_TARGETS.split(",");
    }
    const binaries = {
      [import_fetch_engine.BinaryType.SchemaEngineBinary]: baseDir
    };
    await (0, import_fetch_engine.download)({
      binaries,
      version: import_engines_version.enginesVersion,
      showProgress: true,
      failSilent: true,
      binaryTargets
    }).catch((e) => debug(e));
    cleanupLockFile();
  }
}
function createLockFile() {
  createdLockFile = true;
  import_fs.default.writeFileSync(lockFile, Date.now().toString());
}
function cleanupLockFile() {
  if (createdLockFile) {
    try {
      if (import_fs.default.existsSync(lockFile)) {
        import_fs.default.unlinkSync(lockFile);
      }
    } catch (e) {
      debug(e);
    }
  }
}
main().catch((e) => debug(e));
process.on("beforeExit", () => {
  cleanupLockFile();
});
process.once("SIGINT", () => {
  cleanupLockFile();
  process.exit();
});
