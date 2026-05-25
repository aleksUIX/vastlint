import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const version = process.argv[2];
const stageRootArg = process.argv[3];

if (!version) {
  throw new Error("Usage: node scripts/prepare-js-release-packages.mjs <version> [stage-dir]");
}

if (!/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(version)) {
  throw new Error(`Release version must be a semver string, got '${version}'.`);
}

const scriptDir = path.dirname(new URL(import.meta.url).pathname);
const repoRoot = path.resolve(scriptDir, "..");
const stageRoot = stageRootArg
  ? path.resolve(stageRootArg)
  : path.join(repoRoot, ".release", "js-packages");

const packageConfigs = [
  {
    name: "vastlint",
    sourceDir: path.join(repoRoot, "npm"),
    stageDir: path.join(stageRoot, "vastlint"),
    requiredFiles: ["index.js", "index.d.ts", "README.md"],
    rewriteManifest(packageJson) {
      packageJson.version = version;
      return packageJson;
    },
  },
  {
    name: "vastlint-client",
    sourceDir: path.join(repoRoot, "packages", "vastlint-client"),
    stageDir: path.join(stageRoot, "vastlint-client"),
    requiredFiles: ["dist/index.js", "dist/index.d.ts", "README.md"],
    rewriteManifest(packageJson) {
      packageJson.version = version;
      delete packageJson.private;
      packageJson.dependencies = {
        ...(packageJson.dependencies ?? {}),
        vastlint: `^${version}`,
      };
      return packageJson;
    },
  },
  {
    name: "vastlint-react",
    sourceDir: path.join(repoRoot, "packages", "vastlint-react"),
    stageDir: path.join(stageRoot, "vastlint-react"),
    requiredFiles: ["dist/index.js", "dist/index.d.ts", "README.md"],
    rewriteManifest(packageJson) {
      packageJson.version = version;
      delete packageJson.private;
      packageJson.dependencies = {
        ...(packageJson.dependencies ?? {}),
        "vastlint-client": `^${version}`,
      };
      return packageJson;
    },
  },
];

fs.rmSync(stageRoot, { recursive: true, force: true });
fs.mkdirSync(stageRoot, { recursive: true });

for (const packageConfig of packageConfigs) {
  if (!fs.existsSync(packageConfig.sourceDir)) {
    throw new Error(`Missing source directory for ${packageConfig.name}: ${packageConfig.sourceDir}`);
  }

  fs.cpSync(packageConfig.sourceDir, packageConfig.stageDir, { recursive: true });

  const manifestPath = path.join(packageConfig.stageDir, "package.json");
  const packageJson = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  if (packageJson.name !== packageConfig.name) {
    throw new Error(
      `Expected package name '${packageConfig.name}' at ${manifestPath}, got '${String(packageJson.name)}'.`,
    );
  }

  const rewrittenManifest = packageConfig.rewriteManifest(packageJson);
  fs.writeFileSync(manifestPath, `${JSON.stringify(rewrittenManifest, null, 2)}\n`);

  for (const requiredFile of packageConfig.requiredFiles) {
    const requiredPath = path.join(packageConfig.stageDir, requiredFile);
    if (!fs.existsSync(requiredPath)) {
      throw new Error(`Missing required release file for ${packageConfig.name}: ${requiredPath}`);
    }
  }
}

console.log(`Prepared JS release packages in ${stageRoot}`);