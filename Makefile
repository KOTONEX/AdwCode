# SPDX-License-Identifier: AGPL-3.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode contributors
SHELL := /usr/bin/env bash
.DEFAULT_GOAL := help

.PHONY: help lint typecheck check test build package clean

help:
	@printf '%s\n' \
	  'AdwCode 开发命令' \
	  '' \
	  '  make lint     静态检查（py_compile / package.json / typecheck）' \
	  '  make typecheck 类型检查：mypy（src、tests）+ tsc（extension.js）' \
	  '  make check    校验已生成的主题（键覆盖、未知键、对比度、产品图标）' \
	  '  make test     离线单元测试（unittest，CI 可跑）' \
	  '  make build    生成主题并同步 package.json' \
	  '  make package  打包 adwcode-<版本>.vsix' \
	  '  make clean    清理本地产物（__pycache__ 等）'

lint: typecheck
	python3 -m py_compile src/*.py tests/*.py
	python3 -m json.tool package.json > /dev/null
	@printf '%s\n' '✓ lint 通过'

# 默认宽松：缺 mypy/tsc 时跳过并提示；CI 用 ADWCODE_TYPECHECK_STRICT=1
# 把「工具缺失」变成失败，避免类型检查静默退化。
typecheck:
	@missing=""; \
	if python3 -c 'import mypy' > /dev/null 2>&1; then \
	    python3 -m mypy || exit 1; \
	else \
	    missing="$${missing}mypy "; \
	    printf '%s\n' '（未安装 mypy，跳过 Python 类型检查；安装：pip install mypy）'; \
	fi; \
	if command -v tsc > /dev/null 2>&1; then \
	    tsc -p tsconfig.json || exit 1; \
	else \
	    missing="$${missing}tsc "; \
	    printf '%s\n' '（未安装 tsc，跳过 JS 类型检查；安装：npm i -g typescript）'; \
	fi; \
	if [ -n "$${missing}" ] && [ "$${ADWCODE_TYPECHECK_STRICT:-0}" = "1" ]; then \
	    printf '%s\n' "✗ strict 模式：类型检查工具缺失（$${missing}）" >&2; \
	    exit 1; \
	fi; \
	printf '%s\n' '✓ 类型检查通过'

check:
	python3 src/build.py --check

test:
	python3 -m unittest discover -s tests -p 'test_*.py'

build:
	python3 src/build.py

package:
	python3 src/package.py

clean:
	rm -rf src/__pycache__ tests/__pycache__ __pycache__ .mypy_cache
	rm -f src/*.pyc tests/*.pyc *.pyc
	@printf '%s\n' '✓ 已清理本地产物'
