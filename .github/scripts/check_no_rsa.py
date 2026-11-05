"""Enforce the condition for the RSA advisory exception in .cargo/audit.toml."""

import subprocess
import sys
from pathlib import Path


def main():
    result = subprocess.run(
        [
            "cargo", "tree", "--workspace", "--all-features", "--target", "all",
            "--locked", "--edges", "normal,build,dev", "--prefix", "none",
            "--format", "{p}",
        ],
        cwd=Path(__file__).resolve().parents[2],
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    )
    rsa = sorted({line for line in result.stdout.splitlines() if line.startswith("rsa v")})
    if rsa:
        print(
            "RSA is an active dependency. Remove the RUSTSEC-2023-0071 exception "
            "from .cargo/audit.toml and review its use:\n" + "\n".join(rsa),
            file=sys.stderr,
        )
        return 1
    print("RSA is absent from active dependencies for all workspace features and targets.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
