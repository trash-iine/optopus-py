# Configuration file for the Sphinx documentation builder.
#
# For the full list of built-in configuration values, see the documentation:
# https://www.sphinx-doc.org/en/master/usage/configuration.html

# -- Project information -----------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#project-information

from pathlib import Path

project = "optopus"
copyright = "2026, Trash-iine"
author = "Trash-iine"

# -- General configuration ---------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#general-configuration

extensions = [
    "myst_parser",
    "sphinx.ext.mathjax",
    "sphinx.ext.autodoc",
    "sphinx.ext.napoleon",
]

templates_path = ["_templates"]
exclude_patterns = []

# Render single backticks (used Markdown-style in the Rust /// docstrings) as
# inline code rather than reStructuredText title references.
default_role = "literal"

# -- autodoc -----------------------------------------------------------------
autodoc_default_options = {
    "members": True,
    "undoc-members": False,
}

# -- Options for HTML output -------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#options-for-html-output

html_theme = "furo"
html_static_path = ["_static"]
html_css_files = ["language.css"]

# furo's default sidebar with a link to the other language under the brand.
html_sidebars = {
    "**": [
        "sidebar/brand.html",
        "sidebar/language.html",
        "sidebar/search.html",
        "sidebar/scroll-start.html",
        "sidebar/navigation.html",
        "sidebar/scroll-end.html",
    ]
}

# -- Language switch ---------------------------------------------------------
# The Japanese edition in docs/source-ja mirrors these file names and is published under ja/
# (ADR 0008), so each page links to its counterpart. A page not translated yet links to the other
# edition's top page instead. docs/source-ja/conf.py reuses this file and overrides the context.

DOCS = Path(__file__).resolve().parent.parent


def language_switch(label, code, prefix, other_source):
    """Template context for sidebar/language.html, pointing at the edition in `other_source`."""
    pages = {path.stem for path in (DOCS / other_source).glob("*.md")}
    return {
        "alt_lang_label": label,
        "alt_lang_code": code,
        "alt_lang_prefix": prefix,
        "alt_lang_pages": pages | {"genindex", "search"},
    }


html_context = language_switch("日本語", "ja", "ja/", "source-ja")

# -- MyST parser configuration -----------------------------------------------
myst_enable_extensions = [
    "amsmath",
    "dollarmath",
    "deflist",
    "html_admonition",
    "html_image",
]

# Generate anchors for headings (h1-h3) so in-page links like (#results) resolve.
myst_heading_anchors = 3
