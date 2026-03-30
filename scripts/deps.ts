import { Dependency } from "./deps/types.ts";
import { getFedoraDependencies } from "./deps/linux/fedora.ts";
import { getDebianDependencies } from "./deps/linux/debian.ts";
import { getArchDependencies } from "./deps/linux/arch.ts";
import { getWindowsDependencies } from "./deps/windows/mod.ts";

const colors = {
  reset: "\x1b[0m",
  bright: "\x1b[1m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  cyan: "\x1b[36m",
  red: "\x1b[31m",
};

export async function verifyDependencies(isDev: boolean = false) {
  console.log(`\n${colors.bright}[0]${colors.reset} ${colors.cyan}Checking system dependencies...${colors.reset}`);

  let dependencies: Dependency[] = [];

  if (Deno.build.os === "linux") {
    try {
      const release = await Deno.readTextFile("/etc/os-release");
      if (release.includes("fedora")) {
        console.log(`${colors.cyan}Detected Fedora/RHEL-based system${colors.reset}`);
        dependencies = getFedoraDependencies();
      } else if (release.includes("arch")) {
        console.log(`${colors.cyan}Detected Arch-based system${colors.reset}`);
        dependencies = getArchDependencies();
      } else {
        console.log(`${colors.cyan}Detected Debian/Ubuntu-based system${colors.reset}`);
        dependencies = getDebianDependencies();
      }
    } catch {
      // Fallback
      dependencies = getDebianDependencies();
    }
  } else if (Deno.build.os === "windows") {
    console.log(`${colors.cyan}Detected Windows system${colors.reset}`);
    dependencies = getWindowsDependencies();
  } else {
    console.error(`${colors.red}Unsupported OS: ${Deno.build.os}${colors.reset}`);
    Deno.exit(1);
  }

  const missing: Dependency[] = [];

  for (const dep of dependencies) {
    if (await dep.check()) {
      console.log(`${colors.green}✅ ${dep.name} is installed${colors.reset}`);
    } else {
      console.error(`${colors.red}❌ Missing: ${dep.name}${colors.reset} (${dep.desc})`);
      missing.push(dep);
    }
  }

  if (missing.length > 0) {
    console.log(`\n${colors.bright}${colors.red}Found ${missing.length} missing system dependencies!${colors.reset}`);
    console.log(`${colors.yellow}Please run the following commands to install them:${colors.reset}`);
    
    // Group installation commands for better UX
    const installCommands = new Set(await Promise.all(missing.map(m => m.install())));
    for (const cmd of installCommands) {
      console.log(`${colors.bright}${colors.cyan}${cmd}${colors.reset}`);
    }
    
    Deno.exit(1);
  } else {
    console.log(`\n${colors.green}✅ All system dependencies are installed!${colors.reset}`);
  }
}
