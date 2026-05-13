# Install Argo

This quickstart gets an operator from a fresh machine to a running
worker in about 10 minutes. It does not require a developer toolchain.

## macOS

```bash
brew tap mark-hallam/argo
brew install argo
argo --version
```

Expected output:

```text
argo 0.0.1
```

If you see `command not found: brew`, install Homebrew first from
[brew.sh](https://brew.sh/). If `brew install argo` cannot find the
formula, use the binary release path below until the tap is published
for your account.

## Linux

```bash
curl -fsSL https://raw.githubusercontent.com/mark-hallam/Argo-Agent/main/install.sh | bash
argo --version
```

Expected output:

```text
argo 0.0.1
```

If the installer cannot write to `/usr/local/bin`, re-run it from a
shell that can write there, or move the downloaded `argo` binary to a
directory already on `PATH`.

## Windows

Run Argo from PowerShell:

```powershell
irm https://raw.githubusercontent.com/mark-hallam/Argo-Agent/main/install.ps1 | iex
argo --version
```

Expected output:

```text
argo 0.0.1
```

If PowerShell blocks the script, run it from a trusted terminal session
with script execution enabled for this command.

## Check the command

```bash
argo --help
```

Expected output starts with:

```text
Usage: argo <COMMAND>
```

If this prints a shell `command not found` error, open a new terminal
so the installer-updated `PATH` is loaded.

## Next

Create a project folder in [Initialise a workspace](./init.md).
