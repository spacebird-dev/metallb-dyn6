SHELL := /bin/bash

venv:
	python3 -m venv .venv
	source .venv/bin/activate && pip install -r ../../util/requirements.txt

version: venv
	source .venv/bin/activate && ../../util/version_from_cargo.py --cargo_toml ../../../Cargo.toml

docs: venv
	source .venv/bin/activate && frigate gen . > README.md
