# SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode 贡献者
project('AdwCode', meson_version: '>=1.3.0')

python = import('python').find_installation(get_option('python'))
runtime = run_command(python, '-c',
  'import sys, sysconfig; sys.exit(0 if sys.version_info[:2] == (3, 14) and sysconfig.get_config_var("Py_GIL_DISABLED") == 1 and not sys._is_gil_enabled() else 1)',
  check: false)
if runtime.returncode() != 0
  error('需要 Python 3.14 自由线程版本，且运行时 GIL 必须关闭；请检查 -Dpython 和 PYTHON_GIL。')
endif
root = meson.project_source_root()
dev = files('源码/开发检查.py')

# 主题必须保存在源码目录，供符号链接安装和提交生成产物使用。
run_target('主题', command: [python, root / '源码/生成主题.py'])
run_target('打包', command: [python, root / '源码/打包扩展.py'])
run_target('变更日志', command: [python, root / '源码/生成变更日志.py', '变更日志'])
run_target('发布说明', command: [python, root / '源码/生成变更日志.py', '发布说明'])
run_target('类型检查', command: [python, dev, '类型检查'])
run_target('静态检查', command: [python, dev, '静态检查'])
run_target('格式化', command: [python, dev, '格式化'])
run_target('校验', command: [python, root / '源码/生成主题.py', '--校验'])
run_target('检查样式', command: [python, root / '源码/检查样式.py'])
run_target('性能基准', command: [python, root / '基准/运行基准.py'])

test('静态、格式与类型检查', python, args: [dev, '静态检查'], workdir: root, timeout: 120)
test('主题与图标校验', python, args: [root / '源码/生成主题.py', '--校验'], workdir: root)
test('离线单元测试', python,
  args: ['-m', 'unittest', 'discover', '-s', '测试', '-p', 'test_*.py'],
  workdir: root, timeout: 120)
