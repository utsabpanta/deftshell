class Deftshell < Formula
  desc "AI-Powered Context-Aware Terminal for Developers"
  homepage "https://github.com/deftshell-io/deftshell"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/deftshell-io/deftshell/releases/download/v#{version}/ds-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_ARM64"
    else
      url "https://github.com/deftshell-io/deftshell/releases/download/v#{version}/ds-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_X86"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/deftshell-io/deftshell/releases/download/v#{version}/ds-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_ARM64"
    else
      url "https://github.com/deftshell-io/deftshell/releases/download/v#{version}/ds-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_X86"
    end
  end

  def install
    bin.install "ds"

    # Generate and install shell completions
    generate_completions_from_executable(bin/"ds", "completions")

    # Install man page if present
    man1.install "ds.1" if File.exist?("ds.1")
  end

  def post_install
    ohai "DeftShell installed! Add shell integration to your shell config:"
    ohai "  Zsh:  eval \"$(ds init zsh)\""
    ohai "  Bash: eval \"$(ds init bash)\""
    ohai "  Fish: ds init fish | source"
  end

  test do
    assert_match "DeftShell", shell_output("#{bin}/ds version")
    assert_match "ds", shell_output("#{bin}/ds --help")
  end
end
