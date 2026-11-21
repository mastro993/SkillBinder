# The disk image's window (spec rule 20, ADR 0019): SkillBinder, dmgbuild's built-in arrow, and
# an Applications shortcut to drag it onto. `xtask package` passes the bundle and the volume size
# as defines:
#
#   dmgbuild -s dmgbuild-settings.py -D app=<SkillBinder.app> -D size=<megabytes>m SkillBinder <image>

app = defines["app"]  # noqa: F821
format = "UDZO"
size = defines["size"]  # noqa: F821
files = [(app, "SkillBinder.app")]
symlinks = {"Applications": "/Applications"}
icon_locations = {"SkillBinder.app": (140, 120), "Applications": (500, 120)}
background = "builtin-arrow"
window_rect = ((100, 100), (640, 280))
default_view = "icon-view"
show_status_bar = False
show_toolbar = False
show_pathbar = False
show_sidebar = False
icon_size = 128
text_size = 13
