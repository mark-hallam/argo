# Homebrew formula for argo (the AI-agent platform CLI).
#
# Usage:
#   brew tap mark-hallam/argo
#   brew install argo
#
# This formula is mirrored from the upstream repo and points at the
# release tarballs published by `.github/workflows/release.yml` on
# every `v*` tag. Update `version` + the per-arch `sha256`s for new
# releases; the test block exercises `argo --version` to confirm the
# bottle is well-formed.
class Argo < Formula
  desc "Argo — the ship of agents (AI-agent platform CLI)"
  homepage "https://github.com/mark-hallam/Argo-Agent"
  version "0.0.1"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/mark-hallam/Argo-Agent/releases/download/v#{version}/argo-aarch64-apple-darwin.tar.gz"
      # Replace with the real sha256 once the release tarball ships.
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
    on_intel do
      url "https://github.com/mark-hallam/Argo-Agent/releases/download/v#{version}/argo-x86_64-apple-darwin.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end

  def install
    bin.install "argo"
  end

  def caveats
    <<~EOS
      Quick next steps:
        argo onboard           # first-experience walkthrough
        argo daemon install    # gateway-on-login launchd agent
        argo doctor            # verify the install
    EOS
  end

  test do
    assert_match "argo", shell_output("#{bin}/argo --version")
  end
end
