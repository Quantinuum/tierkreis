# Configuration file for the Sphinx documentation builder.
#
# For the full list of built-in configuration values, see the documentation:
# https://www.sphinx-doc.org/en/master/usage/configuration.html

# -- Project information -----------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#project-information

project = "tierkreis"
copyright = "2025, Quantinuum"
author = "Quantinuum"

# -- General configuration ---------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#general-configuration

extensions = ["autodoc2", "myst_nb"]
autodoc2_packages = [
    "../../tierkreis/tierkreis",
    {
        "path": "../../tierkreis_workers/aer_worker/main.py",
        "module": "aer_worker",
    },
    {
        "path": "../../tierkreis_workers/nexus_worker/main.py",
        "module": "nexus_worker",
    },
    {
        "path": "../../tierkreis_workers/pytket_worker/main.py",
        "module": "pytket_worker",
    },
]
autodoc2_hidden_objects = ["private"]

templates_path = ["_templates"]
nb_execution_excludepatterns = [
    "polling_and_dir.ipynb",
    "storage_and_executors.ipynb",
    "hpc.ipynb",
]


# -- Options for HTML output -------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#options-for-html-output

html_theme = "furo"
html_static_path = ["_static"]
html_theme_options = {"use_download_button": True}
html_favicon = "_static/quantinuum_favicon.svg"

# -- Notebook options --------------------------------------------------------

nb_execution_raise_on_error = True
