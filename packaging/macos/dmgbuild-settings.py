# The disk image's window (spec rule 20, ADR 0019): SkillBinder, the brand's rune arrow, and an
# Applications shortcut to drag it onto, over the background docs/design/brand renders. The volume
# wears the app icon. `xtask package` passes the bundle, the volume size, and the background as
# defines:
#
#   dmgbuild -s dmgbuild-settings.py -D app=<SkillBinder.app> -D size=<megabytes>m \
#     -D background=<dmg-background.png> SkillBinder <image>
#
# dmgbuild finds dmg-background@2x.png beside the background and joins both into a HiDPI TIFF.
import os.path

app = defines["app"]  # noqa: F821
format = "UDZO"
size = defines["size"]  # noqa: F821
files = [(app, "SkillBinder.app")]
symlinks = {"Applications": "/Applications"}
icon = os.path.join(app, "Contents", "Resources", "icon.icns")
icon_locations = {"SkillBinder.app": (160, 200), "Applications": (480, 200)}
background = defines["background"]  # noqa: F821
window_rect = ((100, 100), (640, 360))
default_view = "icon-view"
show_status_bar = False
show_toolbar = False
show_pathbar = False
show_sidebar = False
icon_size = 128
text_size = 13
