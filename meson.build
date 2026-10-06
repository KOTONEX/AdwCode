# SPDX-License-Identifier: AGPL-3.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode contributors
project('AdwCode', meson_version: '>=1.3.0')

python = import('python').find_installation(get_option('python'))
runtime = run_command(python, '-c',
  'import sys, sysconfig; sys.exit(0 if sys.version_info[:2] == (3, 14) and sysconfig.get_config_var("Py_GIL_DISABLED") == 1 and not sys._is_gil_enabled() else 1)',
  check: false)
if runtime.returncode() != 0
  error('需要 Python 3.14 自由线程版本，且运行时 GIL 必须关闭；请检查 -Dpython 和 PYTHON_GIL。')
endif
root = meson.project_source_root()
dev = files('src/dev.py')

# 主题必须保存在源码目录，供符号链接安装和提交生成产物使用。
run_target('themes', command: [python, root / 'src/build.py'])
run_target('package', command: [python, root / 'src/package.py'])
run_target('changelog', command: [python, root / 'src/release_notes.py', 'changelog'])
run_target('release-notes', command: [python, root / 'src/release_notes.py', 'release'])
run_target('typecheck', command: [python, dev, 'typecheck'])
run_target('lint', command: [python, dev, 'lint'])
run_target('format', command: [python, dev, 'format'])
run_target('check', command: [python, root / 'src/build.py', '--check'])
run_target('check-css', command: [python, root / 'src/check_css.py'])
run_target('performance', command: [python, root / 'benchmarks/run.py'])

test('静态、格式与类型检查', python, args: [dev, 'lint'], workdir: root, timeout: 120)
test('主题与图标校验', python, args: [root / 'src/build.py', '--check'], workdir: root)
test('离线单元测试', python,
  args: ['-m', 'unittest', 'discover', '-s', 'tests', '-p', 'test_*.py'],
  workdir: root, timeout: 120)
