SHELL := /bin/bash

venv:
	python3 -m venv .venv
	source .venv/bin/activate && pip install -r ../../util/requirements.txt

set-app-version: venv
	source .venv/bin/activate && ../../util/set_chart_version.py ./ app $(version)

set-chart-version: venv
	source .venv/bin/activate && ../../util/set_chart_version.py ./ chart $(version)

docs: venv
	source .venv/bin/activate && frigate gen . > README.md
