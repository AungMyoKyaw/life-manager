cask "life-manager" do
  version "0.1.0"
  sha256 "REPLACE_WITH_THE_SHA256_OF_THE_DMG"

  url "https://github.com/OWNER/REPOSITORY/releases/download/v#{version}/LifeManager.dmg"
  name "Life Manager"
  desc "A local-first personal productivity app: tasks, calendar, focus, habits and goals."
  homepage "https://github.com/OWNER/REPOSITORY"

  app "Life Manager.app"

  depends_on macos: ">= :monterey"

  zap trash: [
    "~/Library/Application Support/LifeManager",
    "~/Library/Preferences/app.lifemanager.desktop.plist",
    "~/Library/Saved Application State/app.lifemanager.desktop.savedState",
  ]
end
