"""Build and smoke the CLI archive using disposable files, never a user profile."""
import argparse
import io
import json
import os
import pathlib
import signal
import struct
import subprocess
import tarfile
import tempfile
import time

from release_assets import source_version, write_package


def check_elf(data, arch):
    machine = {"x86_64": 62, "aarch64": 183}[arch]
    if len(data) < 20 or data[:6] != b"\x7fELF\x02\x01" or struct.unpack_from("<H", data, 18)[0] != machine:
        raise ValueError("CLI must be a 64-bit little-endian ELF of the requested architecture")


def create_archive(binary, arch, version, source, output, repository):
    source_version(version, source)
    data = binary.read_bytes()
    check_elf(data, arch)
    stem = f"kukuri-cli_{version}_{arch}-unknown-linux-gnu"
    archive = output / f"{stem}.tar.gz"
    if output.exists() and any(output.iterdir()):
        raise ValueError("CLI output directory must be empty")
    output.mkdir(parents=True, exist_ok=True)
    files = {
        "bin/kukuri-cli": (data, 0o755),
        "LICENSE": ((repository / "LICENSE").read_bytes(), 0o644),
        "README.md": ((repository / "docs/runbooks/linux-cli.md").read_bytes(), 0o644),
        "THIRD_PARTY_NOTICES.md": ((repository / "docs/THIRD_PARTY_NOTICES.md").read_bytes(), 0o644),
    }
    with tarfile.open(archive, "w:gz") as tar:
        for name, (content, mode) in files.items():
            info = tarfile.TarInfo(f"{stem}/{name}")
            info.size, info.mode, info.mtime = len(content), mode, 0
            tar.addfile(info, io.BytesIO(content))
    write_package(output, f"cli-linux-{arch}", version, source, [archive.name])
    return archive


def unpack_cli(archive, destination, arch, version):
    stem = f"kukuri-cli_{version}_{arch}-unknown-linux-gnu"
    expected = {f"{stem}/{name}" for name in ("bin/kukuri-cli", "LICENSE", "README.md", "THIRD_PARTY_NOTICES.md")}
    with tarfile.open(archive, "r:gz") as tar:
        members = tar.getmembers()
        if len(members) != len(expected) or {m.name for m in members} != expected or not all(m.isfile() for m in members):
            raise ValueError("CLI archive has missing, duplicate or unsafe entries")
        binary_member = tar.getmember(f"{stem}/bin/kukuri-cli")
        if binary_member.mode & 0o777 != 0o755:
            raise ValueError("CLI archive lost executable mode")
        data = tar.extractfile(binary_member).read()
    check_elf(data, arch)
    binary = destination / "kukuri-cli"
    binary.write_bytes(data)
    binary.chmod(0o755)
    return binary


def smoke(archive, arch, version, runner):
    with tempfile.TemporaryDirectory(prefix="kukuri-cli-smoke-") as temporary:
        root = pathlib.Path(temporary)
        binary = unpack_cli(archive, root, arch, version)
        env = os.environ.copy()
        for name in ("KUKURI_APP_DATA_DIR", "KUKURI_INSTANCE"):
            env.pop(name, None)
        for name, folder in (("XDG_DATA_HOME", "data"), ("XDG_RUNTIME_DIR", "runtime"), ("HOME", "home")):
            path = root / folder
            path.mkdir(mode=0o700)
            env[name] = str(path)
        env["KUKURI_DISABLE_KEYRING"] = "1"
        command = runner + [str(binary)]

        def run(*args, check=True):
            result = subprocess.run(command + list(args), env=env, capture_output=True, text=True, timeout=30)
            if check and result.returncode:
                raise RuntimeError(f"CLI smoke command failed: {args[0]}")
            return result

        if run("--version").stdout.strip() != f"kukuri-cli {version}":
            raise ValueError("CLI binary version mismatch")
        # Synthetic test identity/consent only; no host account or keyring is used.
        prefix = ("--profile", "release-smoke")
        run(*prefix, "consent", "accept", "--accept-documents", "--age-confirmed")
        process = subprocess.Popen(command + list(prefix) + ["daemon", "run"], env=env,
                                   stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        try:
            deadline = time.monotonic() + 60
            while True:
                if process.poll() is not None:
                    raise RuntimeError("Packaged daemon exited before readiness")
                status = run(*prefix, "call", "client.status", check=False)
                if status.returncode == 0:
                    value = json.loads(status.stdout)
                    if value.get("ok") and value["data"].get("ready") is True:
                        break
                if time.monotonic() >= deadline:
                    raise RuntimeError("Packaged daemon readiness timed out")
                time.sleep(0.2)
            schema = json.loads(run(*prefix, "call", "protocol.schema").stdout)
            if not schema.get("ok") or schema["data"]["protocol"]["$defs"]["request"]["type"] != "object":
                raise ValueError("Packaged CLI schema response is invalid")
            process.send_signal(signal.SIGTERM)
            process.wait(timeout=20)
            if process.returncode != 0:
                raise RuntimeError("Packaged daemon did not stop cleanly")
            if list((root / "runtime").rglob("*.sock")):
                raise RuntimeError("Daemon socket remained after shutdown")
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
            process.stderr.close()
    print(json.dumps({"arch": arch, "version": version, "schema": "passed", "status": "passed", "shutdown": "passed"}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["package", "smoke"])
    parser.add_argument("--arch", choices=["x86_64", "aarch64"], required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--source")
    parser.add_argument("--binary", type=pathlib.Path)
    parser.add_argument("--output", type=pathlib.Path)
    parser.add_argument("--archive", type=pathlib.Path)
    parser.add_argument("--runner", nargs=argparse.REMAINDER, default=[])
    args = parser.parse_args()
    if args.mode == "package":
        create_archive(args.binary, args.arch, args.version, args.source, args.output, pathlib.Path(__file__).resolve().parents[2])
    else:
        smoke(args.archive, args.arch, args.version, args.runner)
