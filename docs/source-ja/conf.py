# Sphinx configuration of the Japanese edition (ADR 0008).
#
# Every setting comes from the English edition's docs/source/conf.py; only what differs by
# language is overridden here.

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "source"))

from conf import *  # noqa: F403 -- the English settings, overridden below
from conf import language_switch

language = "ja"

# Paths in conf.py are relative to the directory of the conf.py Sphinx loads, so point the
# shared templates and static files back at the English edition's.
templates_path = ["../source/_templates"]
html_static_path = ["../source/_static"]

# Published under ja/, one level below the English edition.
html_context = language_switch("English", "en", "../", "source")
