# 性能基准

基准用于 Linux 上的开发性能回归，不属于扩展运行时，也不在 CI 中自动执行。
以重复测量结果判断变化，不设置跨机器通用的耗时门槛。

## 命令行与扩展逻辑

需要完整 Git 历史与版本标签、Rust 工具链（`cargo build`）、Node.js、GNOME 的
`gsettings` 和可自动定位的本机 VS Code 样式表。找不到样式表时基准会失败，
不把跳过校验记为成功样本：

```sh
cargo run --quiet -- 性能基准
```

默认每个命令预热 2 次，采集 20 个新进程样本，保留操作系统文件缓存。
结果写入 `builddir/performance.json`。也可以增加样本数：

```sh
cargo run --quiet -- 性能基准 --次数 50 --输出 builddir/performance.json
```

`性能基准` 在临时副本中运行主题、校验、检查样式与打包，默认使用阻塞等待测墙钟，
不读取每个子进程的 `/proc`、CPU 时间或 RSS。需要旧资源测量时显式启用：

```sh
cargo run --quiet -- 性能基准 --详细资源 --次数 20 --输出 builddir/详细资源.json
```

报告格式版本2保留样本与摘要字段；未启用或未采集到的资源值为 null，不能当成0。
详细模式的 VmHWM 每0.5ms轮询，仍可能漏掉短进程最终峰值，测量自身也会增加干扰。
不同模式的墙钟结果不直接混用。
准备阶段从 Git 生成固定的 `builddir/CHANGELOG.md`，临时副本显式使用该日志打包。
生成日志的耗时不计入 VSIX 打包样本，临时副本也不复制 Git 仓库。


`扩展基准.cjs` 使用真实临时文件和模拟 VS Code API，测量脚本加载、扩展激活、
安装状态读取、状态输出生成。禁止配置写入、外部命令与子进程。
Node 仅输出原始样本；Rust 校验场景、毫秒单位、数量与有限非负数，统一计算
中位数、最近秩 P95、最小值和最大值，最终报告仍保留原有 `summary` 与 `samples`
字段。直接运行 CJS 得到的只有原始采样报告；完整摘要应通过 `性能基准` 生成。
这些数值不能代替真实扩展宿主的完整启动耗时。

## 独立工作台

需要本机 Linux VS Code。Playwright 只用于测试，可安装在仓库之外：

```sh
npm install --prefix /tmp/adwcode-performance-tools playwright-core
cargo run --quiet -- 工作台基准 \
  --浏览器工具 /tmp/adwcode-performance-tools/node_modules/playwright-core \
  --输出 builddir/performance-ui.json
```

该命令新建临时用户数据、扩展目录和测试工作区，仅复制本项目扩展。
如需指定安装路径，可传入 `--编辑器程序 /路径/bin/code`。
不执行窗口重载或加载器命令，不修改 VS Code 安装文件或操作者配置；结束后关闭
测试窗口并清理临时目录。独立窗口可能暂时取得焦点，测试期间避免向它输入内容。

在同一套 AdwCode 主题和产品图标下，交替比较附加 CSS 的启用与关闭。
安装目录中已经注入的项目样式先从测试窗口的 DOM 中移除，随后注入当前源码；
这一步只影响测试窗口。场景包括非活动状态切换、实际编辑器滚动和 1000 个
非虚拟侧栏行的压力测试，每个场景每种样式预热 1 次并采样 6 次。
窗口状态脚本在各组均启用；每次状态切换后等待一个微任务，保证观察器已经同步，
不会把尚未应用的样式当成优化结果。界面断言检查正文、高对比度保护、新增导航、
重复注入、启动等待和观察器释放；有参考 CSS 时还对照实际计算样式。

修改前后的配对比较可保存原样式并传入参考文件：

```sh
git show c36fdb5:extras/gnome-look.css > builddir/修改前外观.css
cargo run --quiet -- 工作台基准 \
  --浏览器工具 /tmp/adwcode-performance-tools/node_modules/playwright-core \
  --参考样式 builddir/修改前外观.css \
  --输出 builddir/performance-fixed-ui.json
```

追加 `--场景 scroll --次数 8` 可单独复核滚动，输出到另一个结果文件。

如需确认非活动样式这一组规则的开销：

```sh
cargo run --quiet -- 工作台基准 \
  --浏览器工具 /tmp/adwcode-performance-tools/node_modules/playwright-core \
  --仅选择器 --输出 builddir/performance-selectors.json
```

该对照在原生样式、完整 CSS 和临时剔除非活动规则的 CSS 之间交替，不修改源码。
剔除规则同时取消了对应视觉变化，不能把全部收益解释为 `:has()` 语法自身的开销。

CDP 记录的是渲染器任务、脚本、样式重算和布局计时。滚动样本中的 `frames`
是串行自动化输入后的 RAF 采样间隔，会受协议调用影响，不能当作应用帧率或掉帧数据。
结果的 `complete` 必须为 `true`，失败或中断留下的部分样本不能作为完整结果。

本机结果与原始样本见 [性能测试](../文档/07-性能测试.md)。

4.4.0 起加载场景为“扩展加载（显式宿主接口）”，测试与基准共用 `测试/扩展宿主.cjs`；
它包含真实入口的 VM 加载，不能与 4.3.0 的单独服务工厂场景直接比较。
