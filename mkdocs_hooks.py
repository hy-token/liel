"""MkDocs hooks — inject Google Search Console verification meta into built homepage.

Avoids ``theme.custom_dir`` / an ``overrides/`` directory so CI passes when that
folder is absent from a synced checkout (same ``mkdocs.yml`` everywhere).
Requires MkDocs >= 1.6 (hooks support).
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

_GOOGLE_SITE_VERIFICATION = "z2r9ze4tX3UblRKzwdSRwrSOvv-34UzkKajlBnxh408"


def on_post_build(config: dict[str, Any], **kwargs: Any) -> None:
    site_dir = Path(config["site_dir"])
    index = site_dir / "index.html"
    if not index.is_file():
        return
    text = index.read_text(encoding="utf-8")
    if "google-site-verification" in text:
        return
    meta = (
        f'    <meta name="google-site-verification" content="{_GOOGLE_SITE_VERIFICATION}" />\n'
    )
    needle = "<head>"
    if needle not in text:
        return
    text = text.replace(needle, needle + "\n" + meta, 1)
    index.write_text(text, encoding="utf-8")
