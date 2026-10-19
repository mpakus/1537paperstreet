#!/usr/bin/env python3
"""Verify both CLI directions with public synthetic data in an isolated directory.

Usage: python3 scripts/test-age-interop.py /path/to/age /path/to/rage
Requires a POSIX pseudo-terminal, Python 3, and a built encryption_probe example.
No test password or decrypted fixture is printed.
"""
import os
import pathlib
import pty
import select
import sys
import tempfile
import time
import subprocess

PASSWORD = b"paperstreet interoperability test only\n"
PROBE = pathlib.Path(os.environ.get("PAPERSTREET_ENCRYPTION_PROBE", str(
    pathlib.Path(__file__).resolve().parent.parent / "target/release/examples/encryption_probe"
)))


def run_cli(command):
    pid, fd = pty.fork()
    if pid == 0:
        os.environ["PINENTRY_PROGRAM"] = "/nonexistent-paperstreet-test-pinentry"
        os.execv(command[0], command)
    pending = b""
    deadline = time.monotonic() + 60
    try:
        while time.monotonic() < deadline:
            if select.select([fd], [], [], 0.2)[0]:
                try:
                    chunk = os.read(fd, 4096)
                except OSError:
                    break
                if not chunk:
                    break
                pending += chunk
                if b":" in pending and b"passphrase" in pending.lower():
                    os.write(fd, PASSWORD)
                    pending = b""
        else:
            os.kill(pid, 9)
            raise RuntimeError("CLI passphrase prompt timed out")
    finally:
        os.close(fd)
        _, status = os.waitpid(pid, 0)
    if status != 0:
        raise RuntimeError("Independent CLI failed (output withheld)")


def main():
    if len(sys.argv) != 3:
        raise SystemExit("Provide absolute paths to age and rage")
    with tempfile.TemporaryDirectory(prefix="paperstreet-interop-") as temporary:
        directory = pathlib.Path(temporary)
        subprocess.run([str(PROBE), "emit", temporary], check=True)
        for index, binary in enumerate(sys.argv[1:]):
            output = directory / f"cli-{index}.md"
            cipher = directory / f"cli-{index}.md.age"
            run_cli([binary, "-d", "-o", str(output), str(directory / "app.md.age")])
            assert output.read_bytes() == (directory / "fixture.md").read_bytes()
            run_cli([binary, "-p", "-o", str(cipher), str(directory / "fixture.md")])
            subprocess.run([str(PROBE), "verify", str(cipher)], check=True)
            print(f"{pathlib.Path(binary).name}: app → CLI and CLI → app passed")


if __name__ == "__main__":
    main()
