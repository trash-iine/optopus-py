# Configuration file for the Sphinx documentation builder.
#
# For the full list of built-in configuration values, see the documentation:
# https://www.sphinx-doc.org/en/master/usage/configuration.html

# -- Project information -----------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#project-information

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
    # Emits `.nojekyll` so GitHub Pages serves `_static/` instead of treating
    # the underscore prefix as a Jekyll internal.
    "sphinx.ext.githubpages",
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
