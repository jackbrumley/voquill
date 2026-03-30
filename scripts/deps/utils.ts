export async function isCommandInPath(cmd: string): Promise<boolean> {
  try {
    const process = new Deno.Command("which", {
      args: [cmd],
      stdout: "null",
      stderr: "null",
    });
    const { success } = await process.output();
    if (success) return true;
  } catch {
    // continue
  }

  // Fallback check for cargo in ~/.cargo/bin
  if (cmd === "cargo") {
    const home = Deno.env.get("HOME");
    if (home) {
      const cargoPath = `${home}/.cargo/bin/cargo`;
      try {
        await Deno.stat(cargoPath);
        return true;
      } catch {
        return false;
      }
    }
  }

  return false;
}

export async function checkPkgConfig(pkg: string): Promise<boolean> {
  try {
    if (!await isCommandInPath("pkg-config")) return false;
    const process = new Deno.Command("pkg-config", {
      args: ["--exists", pkg],
      stdout: "null",
      stderr: "null",
    });
    return (await process.output()).success;
  } catch {
    return false;
  }
}

export async function checkRpm(pkg: string): Promise<boolean> {
  try {
    const process = new Deno.Command("rpm", {
      args: ["-q", pkg],
      stdout: "null",
      stderr: "null",
    });
    return (await process.output()).success;
  } catch {
    return false;
  }
}

export async function checkDeb(pkg: string): Promise<boolean> {
  try {
    const process = new Deno.Command("dpkg", {
      args: ["-s", pkg],
      stdout: "null",
      stderr: "null",
    });
    return (await process.output()).success;
  } catch {
    return false;
  }
}

export async function checkPacman(pkg: string): Promise<boolean> {
  try {
    const process = new Deno.Command("pacman", {
      args: ["-Qq", pkg],
      stdout: "null",
      stderr: "null",
    });
    return (await process.output()).success;
  } catch {
    return false;
  }
}
