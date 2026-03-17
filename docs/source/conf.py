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
add_module_names = False
extensions = ["autodoc2", "myst_nb", "sphinx.ext.intersphinx", "sphinx.ext.mathjax"]
latex_engine = "xelatex"
autodoc2_packages = [
    "../../tierkreis/tierkreis",
    {
        "path": "../../tierkreis_workers/aer_worker/tkr_aer_worker_impl/impl.py",
        "module": "aer_worker",
    },
    {
        "path": "../../tierkreis_workers/ibmq_worker/tkr_ibmq_worker_impl/impl.py",
        "module": "ibmq_worker.tkr_ibmq_worker_impl.impl",
    },
    {
        "path": "../../tierkreis_workers/nexus_worker/tkr_nexus_worker_impl/impl.py",
        "module": "nexus_worker",
    },
    {
        "path": "../../tierkreis_workers/pytket_worker/tkr_pytket_worker_impl/impl.py",
        "module": "pytket_worker.tkr_pytket_worker_impl.impl",
    },
    {
        "path": "../../tierkreis_workers/quantinuum_worker/tkr_quantinuum_worker_impl/impl.py",
        "module": "quantinuum_worker.tkr_quantinuum_worker_impl.impl",
    },
    {
        "path": "../../tierkreis_workers/qulacs_worker/tkr_qulacs_worker_impl/impl.py",
        "module": "qulacs_worker",
    },
]
autodoc2_hidden_objects = ["private"]
source_suffix = {
    ".rst": "restructuredtext",
    ".ipynb": "myst-nb",
    ".myst": "myst-nb",
}

templates_path = ["_templates"]
nb_execution_excludepatterns = [
    "polling_and_dir.ipynb",
    "storage_and_executors.ipynb",
    "hpc.ipynb",
]
myst_enable_extensions = ["dollarmath", "amsmath"]

nitpicky = True
exclude_patterns = ["_build", "Thumbs.db", ".DS_Store", "examples/**/.venv/**"]


suppress_warnings = ["ref.python", "ref.class"]
intersphinx_mapping = {
    "python": ("https://docs.python.org/3", None),
    "typing_extensions": ("https://typing-extensions.readthedocs.io/en/latest/", None),
    "pydantic": ("https://docs.pydantic.dev/latest/", None),
    "pytket": ("https://docs.quantinuum.com/tket/api-docs/", None),
    "pytket.extensions.qiskit": (
        "https://docs.quantinuum.com/tket/extensions/pytket-qiskit/",
        None,
    ),
    "qnexus": ("https://docs.quantinuum.com/nexus/", None),
}
# -- Options for HTML output -------------------------------------------------
# https://www.sphinx-doc.org/en/master/usage/configuration.html#options-for-html-output

html_theme = "furo"
html_static_path = ["_static"]
html_favicon = "_static/quantinuum_favicon.svg"

# -- Notebook options --------------------------------------------------------

nb_execution_raise_on_error = True
