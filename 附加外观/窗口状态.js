// SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-MulanPubL-2.0-or-later
// SPDX-FileCopyrightText: 2026 AdwCode 贡献者
// 只同步原生标题栏的窗口状态，不修改菜单位置、正文或窗口焦点。
// @ts-check
(() => {
  const state = /** @type {{[key: symbol]: (() => void) | undefined}} */ (/** @type {unknown} */ (window));
  const key = Symbol.for("adwcode.windowState");
  state[key]?.();
  /** @type {Set<Element>} */
  const marked = new Set();
  /** @type {MutationObserver | undefined} */
  let observer;
  /** @type {MutationObserver | undefined} */
  let layoutObserver;
  const clear = () => {
    observer?.disconnect();
    layoutObserver?.disconnect();
    for (const part of marked) part.classList.remove("adwcode-window-inactive");
    marked.clear();
    window.removeEventListener("pagehide", clear);
    delete state[key];
  };
  state[key] = clear;
  window.addEventListener("pagehide", clear, { once: true });
  const bind = () => {
    const workbench = document.querySelector(".monaco-workbench");
    const titlebar = workbench?.querySelector(".part.titlebar");
    if (!workbench || !titlebar || !workbench.querySelector(".part.editor")) return false;
    observer?.disconnect();
    const update = () => {
      const inactive = titlebar.classList.contains("inactive");
      for (const part of marked) if (!part.isConnected) marked.delete(part);
      // 只标记导航容器，新增标签和按钮继承状态；正文不参与状态类切换。
      for (const part of workbench.querySelectorAll(".part.editor .tabs-container, .part.editor .editor-actions, .part.sidebar > .title, .part.auxiliarybar > .title, .part.panel > .title, .part.activitybar")) {
        part.classList.toggle("adwcode-window-inactive", inactive);
        marked.add(part);
      }
      layoutObserver?.disconnect();
      if (inactive) {
        // 非活动时若布局新建导航容器，补充状态；文本行和列表内容变化直接忽略。
        layoutObserver ??= new MutationObserver((records) => {
          if (records.some((record) => record.target instanceof Element && record.target.matches(".monaco-workbench, .monaco-grid-view, .monaco-grid-branch-node, .split-view-container, .split-view-view, .editor-group-container, .title, .tabs-and-actions-container, .part"))) update();
        });
        layoutObserver.observe(workbench, { childList: true, subtree: true });
      }
    };
    observer = new MutationObserver(update);
    observer.observe(titlebar, { attributes: true, attributeFilter: ["class"] });
    update();
    return true;
  };
  if (!bind()) {
    // 只在启动时等候工作台创建；绑定完成后不再监听文档树。
    observer = new MutationObserver(bind);
    observer.observe(document.documentElement, { childList: true, subtree: true });
  }
})();
