cask "skillbinder@beta" do
  version "@VERSION@"
  sha256 "@SHA256@"

  url "https://github.com/mastro993/SkillBinder/releases/download/v#{version}/skillbinder-#{version}-universal-apple-darwin.dmg"
  name "SkillBinder Beta"
  desc "Manage a local library of agent skills"
  homepage "https://github.com/mastro993/SkillBinder"

  livecheck do
    url "https://mastro993.github.io/SkillBinder/updates/v1/beta.json"
    strategy :json do |json|
      json["version"]
    end
  end

  auto_updates true
  conflicts_with cask: "skillbinder"
  depends_on macos: :tahoe

  app "SkillBinder.app"
  # The `skillbinder` command (ADR 0024); Settings reads it as installed by Homebrew.
  binary "#{appdir}/SkillBinder.app/Contents/MacOS/skillbinder-cli", target: "skillbinder"

  uninstall quit: "com.skillbinder.desktop"

  zap trash: [
    "~/Library/Application Support/SkillBinder",
    "~/Library/Preferences/com.skillbinder.desktop.plist",
  ]
end
