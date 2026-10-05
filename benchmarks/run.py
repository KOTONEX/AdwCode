#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode contributors
"""Linux 离线性能基准：临时副本中运行命令，输出耗时、CPU 时间与峰值 RSS。"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import platform
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent


def summary(values: list[float]) -> dict[str, float]:
    ordered = sorted(values)
    return {
        "median": statistics.median(values),
        "p95": ordered[min(len(ordered) - 1, math.ceil(0.95 * len(ordered)) - 1)],
        "min": ordered[0],
        "max": ordered[-1],
    }


def sample(command: list[str], cwd: Path) -> dict[str, float]:
    with tempfile.TemporaryFile() as output:
        start = time.perf_counter()
        previous = Path.cwd()
        try:
            os.chdir(cwd)
            # 直接启动程序，不经过 shell。
            pid = os.posix_spawn(command[0], command, os.environ, file_actions=[
                (os.POSIX_SPAWN_DUP2, output.fileno(), 1),
                (os.POSIX_SPAWN_DUP2, output.fileno(), 2),
            ])
        finally:
            os.chdir(previous)
        # wait4 的峰值可能包含启动阶段继承的父进程内存；另采样 exec 后的 VmHWM。
        executable = str(Path(command[0]).resolve())
        observed_rss = 0
        while True:
            try:
                if os.readlink(f"/proc/{pid}/exe") == executable:
                    status_text = Path(f"/proc/{pid}/status").read_text(encoding="utf-8")
                    high_water = next((line for line in status_text.splitlines() if line.startswith("VmHWM:")), "")
                    if high_water:
                        observed_rss = max(observed_rss, int(high_water.split()[1]))
            except (OSError, ValueError):
                pass
            finished, status, usage = os.wait4(pid, os.WNOHANG)
            if finished:
                break
            time.sleep(0.0005)
        elapsed = (time.perf_counter() - start) * 1000
        if os.waitstatus_to_exitcode(status):
            output.seek(0)
            raise RuntimeError(output.read().decode(errors="replace"))
    if not observed_rss:
        raise RuntimeError("未采集到被测程序的 VmHWM")
    return {"wall_ms": elapsed, "cpu_ms": (usage.ru_utime + usage.ru_stime) * 1000, "rss_mib": observed_rss / 1024}


def fingerprint() -> dict[str, str]:
    result: dict[str, str] = {}
    for folder in ("themes", "extension", "extras"):
        for path in sorted((ROOT / folder).rglob("*")):
            if path.is_file():
                result[str(path.relative_to(ROOT))] = hashlib.sha256(path.read_bytes()).hexdigest()
    result["package.json"] = hashlib.sha256((ROOT / "package.json").read_bytes()).hexdigest()
    return result


def idle_monitor() -> dict[str, float]:
    """短时间观测真实 gsettings 监听器，完成后立即停止。"""
    gsettings = shutil.which("gsettings")
    if not gsettings:
        raise RuntimeError("缺少 gsettings")
    process = subprocess.Popen([gsettings, "monitor", "org.gnome.desktop.interface", "accent-color"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        time.sleep(0.2)
        if process.poll() is not None:
            raise RuntimeError("GNOME 强调色监听器未能启动")
        def ticks() -> int:
            fields = Path(f"/proc/{process.pid}/stat").read_text(encoding="utf-8").rsplit(")", 1)[1].split()
            return int(fields[11]) + int(fields[12])
        initial = ticks()
        time.sleep(3)
        elapsed_ticks = ticks() - initial
        clock_ticks = os.sysconf("SC_CLK_TCK")
        status = Path(f"/proc/{process.pid}/status").read_text(encoding="utf-8")
        rss = int(next(line.split()[1] for line in status.splitlines() if line.startswith("VmRSS:")))
        return {"interval_s": 3, "cpu_ms": elapsed_ticks * 1000 / clock_ticks, "timer_resolution_ms": 1000 / clock_ticks, "rss_mib": rss / 1024}
    finally:
        process.terminate()
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runs", type=int, default=20, help="每个命令的有效样本数")
    parser.add_argument("--output", type=Path, default=ROOT / "builddir/performance.json", help="原始结果 JSON")
    args = parser.parse_args()
    if sys.platform != "linux" or args.runs < 5:
        parser.error("需要 Linux，且 --runs 至少为 5")
    before = fingerprint()
    result: dict[str, Any] = {
        "schema": 1,
        "revision": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "version": json.loads((ROOT / "package.json").read_text(encoding="utf-8"))["version"],
        "environment": {"platform": platform.platform(), "python": sys.version, "gil_enabled": getattr(sys, "_is_gil_enabled", lambda: True)()},
        "method": {"warmup": 2, "runs": args.runs, "p95": "最近秩：第 ceil(0.95*n) 个有序样本", "cache": "保留操作系统文件缓存，每个样本使用新进程", "unit": "毫秒；RSS 为 MiB", "rss": "exec 后每 0.5ms 采样 /proc/VmHWM；短进程可能漏掉最终峰值"},
        "commands": [],
    }
    result["environment"]["cpu"] = next(line.split(":", 1)[1].strip() for line in Path("/proc/cpuinfo").read_text(encoding="utf-8").splitlines() if line.startswith("model name"))
    result["environment"]["memory_kib"] = int(next(line.split()[1] for line in Path("/proc/meminfo").read_text(encoding="utf-8").splitlines() if line.startswith("MemTotal:")))
    with tempfile.TemporaryDirectory(prefix="adwcode-performance-") as directory:
        clone = Path(directory)
        for name in ["src", "themes", "product-icons", "extension", "extras", "assets", "docs"]:
            shutil.copytree(ROOT / name, clone / name, ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
        for name in ["package.json", "README.md", "LICENSE", "CONTRIBUTING.md", "CHANGELOG.md", "AGENTS.md"]:
            shutil.copy2(ROOT / name, clone / name)
        python = sys.executable
        commands = [
            ("Python 空进程", [python, "-c", "pass"]),
            ("默认构建（10 个主题，不读取系统）", [python, "src/build.py", "--no-system"]),
            ("全部强调色构建（26 个主题）", [python, "src/build.py", "--accents", "all"]),
            ("默认主题校验", [python, "src/build.py", "--check"]),
            ("CSS 兼容校验", [python, "src/check_css.py"]),
            ("VSIX 打包", [python, "src/package.py"]),
            ("读取 GNOME 强调色", [shutil.which("gsettings") or "/usr/bin/gsettings", "get", "org.gnome.desktop.interface", "accent-color"]),
        ]
        for title, command in commands:
            if title == "默认主题校验":
                sample([python, "src/build.py", "--no-system"], clone)
            for _ in range(2):
                sample(command, clone)
            samples = [sample(command, clone) for _ in range(args.runs)]
            entry = {"name": title, "command": [Path(command[0]).name, *command[1:]], "samples": samples}
            entry["summary"] = {key: summary([item[key] for item in samples]) for key in samples[0]}
            result["commands"].append(entry)
            print(f"{title}: 中位 {entry['summary']['wall_ms']['median']:.2f} ms，P95 {entry['summary']['wall_ms']['p95']:.2f} ms，峰值 RSS {entry['summary']['rss_mib']['max']:.1f} MiB", flush=True)
        assert before == fingerprint(), "基准意外改动了源码或主题"
        node = shutil.which("node")
        if not node:
            raise RuntimeError("扩展基准需要 Node.js")
        extension = subprocess.run([node, "--expose-gc", str(ROOT / "benchmarks/extension.cjs"), str(clone)], check=True, capture_output=True, text=True)
        result["extension"] = json.loads(extension.stdout)
        print("扩展离线基准完成", flush=True)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    result["accent_monitor_idle"] = idle_monitor()
    assert before == fingerprint(), "基准意外改动了源码或主题"
    args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"原始结果：{args.output}")


if __name__ == "__main__":
    main()
