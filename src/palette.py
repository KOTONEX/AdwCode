# SPDX-License-Identifier: AGPL-3.0-or-later
# SPDX-FileCopyrightText: 2026 AdwCode contributors
"""AdwCode 的 Adwaita 颜色角色。

来源
----
- libadwaita 1.10 CSS 变量（GNOME 51）：
  https://gnome.pages.gitlab.gnome.org/libadwaita/doc/1.10/css-variables.html
  取值已与系统 `libadwaita-1.so.0` 中提取的 `/org/gnome/Adwaita/styles/gtk.css`
  逐项核对。
- GtkSourceView 5 的 `Adwaita` / `Adwaita-dark` 样式方案（LGPL-2.1-or-later），
  存放于 `src/gtksourceview_xml/`，用于编辑器默认值。

8 位十六进制颜色写作 `#rrggbbaa`；不透明颜色写作 `#rrggbb`。
"""

from __future__ import annotations

import re
from typing import Literal, TypedDict


class BaseColors(TypedDict):
    """单个明暗模式的基础颜色表（``LIGHT`` / ``DARK``）。"""

    window_bg: str
    view_bg: str
    headerbar_bg: str
    headerbar_backdrop: str
    sidebar_bg: str
    sidebar_backdrop: str
    secondary_sidebar_bg: str
    secondary_sidebar_backdrop: str
    card_bg: str
    popover_bg: str
    dialog_bg: str
    overview_bg: str
    thumbnail_bg: str
    active_toggle_bg: str
    fg: str
    shade: str
    card_shade: str
    headerbar_shade: str
    headerbar_darker_shade: str
    sidebar_shade: str
    sidebar_border: str
    secondary_sidebar_shade: str
    secondary_sidebar_border: str
    popover_shade: str
    scrollbar_outline: str
    border_opacity: float
    dim_opacity: float
    disabled_opacity: float
    destructive: tuple[str, str, str]
    success: tuple[str, str, str]
    warning: tuple[str, str, str]


ACCENT_NAMES: list[str] = [
    "blue",
    "teal",
    "green",
    "yellow",
    "orange",
    "red",
    "pink",
    "purple",
    "slate",
]

#: 强调色的中文名，用于主题标签与命令提示。
ACCENT_LABELS: dict[str, str] = {
    "blue": "蓝色",
    "teal": "青色",
    "green": "绿色",
    "yellow": "黄色",
    "orange": "橙色",
    "red": "红色",
    "pink": "粉色",
    "purple": "紫色",
    "slate": "石板灰",
}

#: 明暗模式的中文名。
MODE_LABELS: dict[str, str] = {"dark": "深色", "light": "浅色"}

#: 名称 -> （背景色, 浅色 standalone, 深色 standalone）；前景色统一为白色。
ACCENT_COLORS: dict[str, tuple[str, str, str]] = {
    "blue": ("#3584e4", "#0461be", "#81d0ff"),
    "teal": ("#2190a4", "#007184", "#7bdff4"),
    "green": ("#3a944a", "#15772e", "#8de698"),
    "yellow": ("#c88800", "#905300", "#ffc057"),
    "orange": ("#ed5b00", "#b62200", "#ff9c5b"),
    "red": ("#e62d42", "#c00023", "#ff888c"),
    "pink": ("#d56199", "#a2326c", "#ffa0d8"),
    "purple": ("#9141ac", "#8939a4", "#fba7ff"),
    "slate": ("#6f8396", "#526678", "#bbd1e5"),
}

#: GNOME HIG 调色板，1（最浅）到 5（最深）。
PALETTE: dict[str, dict[int, str]] = {
    "blue": {1: "#99c1f1", 2: "#62a0ea", 3: "#3584e4", 4: "#1c71d8", 5: "#1a5fb4"},
    "green": {1: "#8ff0a4", 2: "#57e389", 3: "#33d17a", 4: "#2ec27e", 5: "#26a269"},
    "yellow": {1: "#f9f06b", 2: "#f8e45c", 3: "#f6d32d", 4: "#f5c211", 5: "#e5a50a"},
    "orange": {1: "#ffbe6f", 2: "#ffa348", 3: "#ff7800", 4: "#e66100", 5: "#c64600"},
    "red": {1: "#f66151", 2: "#ed333b", 3: "#e01b24", 4: "#c01c28", 5: "#a51d2d"},
    "purple": {1: "#dc8add", 2: "#c061cb", 3: "#9141ac", 4: "#813d9c", 5: "#613583"},
    "brown": {1: "#cdab8f", 2: "#b5835a", 3: "#986a44", 4: "#865e3c", 5: "#63452c"},
    "light": {1: "#ffffff", 2: "#f6f5f4", 3: "#deddda", 4: "#c0bfbc", 5: "#9a9996"},
    "dark": {1: "#77767b", 2: "#5e5c64", 3: "#3d3846", 4: "#241f31", 5: "#000000"},
    "teal": {1: "#93ddc2", 2: "#5bc8af", 3: "#33b2a4", 4: "#26a1a2", 5: "#218787"},
    # pink 与 slate 在 libadwaita 中只作为强调色存在，中间色阶由强调色背景
    # 与 standalone 颜色推导得到。
    "pink": {1: "#ffa0d8", 2: "#d56199", 3: "#bb5587", 4: "#a2326c", 5: "#7a2651"},
    "slate": {1: "#bbd1e5", 2: "#8c9cab", 3: "#6f8396", 4: "#526678", 5: "#3e4d5a"},
}

#: 由 GNOME 调色板推导的 16 个终端颜色（明暗模式相同）。
ANSI: dict[str, str] = {
    "terminal.ansiBlack": PALETTE["dark"][4],
    "terminal.ansiRed": PALETTE["red"][4],
    "terminal.ansiGreen": PALETTE["green"][5],
    "terminal.ansiYellow": PALETTE["yellow"][5],
    "terminal.ansiBlue": PALETTE["blue"][4],
    "terminal.ansiMagenta": PALETTE["purple"][4],
    "terminal.ansiCyan": "#2190a4",
    "terminal.ansiWhite": PALETTE["light"][4],
    "terminal.ansiBrightBlack": PALETTE["dark"][2],
    "terminal.ansiBrightRed": PALETTE["red"][2],
    "terminal.ansiBrightGreen": PALETTE["green"][2],
    "terminal.ansiBrightYellow": PALETTE["yellow"][2],
    "terminal.ansiBrightBlue": PALETTE["blue"][2],
    "terminal.ansiBrightMagenta": PALETTE["purple"][2],
    "terminal.ansiBrightCyan": PALETTE["teal"][2],
    "terminal.ansiBrightWhite": PALETTE["light"][2],
}

LIGHT: BaseColors = {
    "window_bg": "#fafafb",
    "view_bg": "#ffffff",
    "headerbar_bg": "#ffffff",
    "headerbar_backdrop": "#fafafb",
    "sidebar_bg": "#ebebed",
    "sidebar_backdrop": "#f2f2f4",
    "secondary_sidebar_bg": "#f3f3f5",
    "secondary_sidebar_backdrop": "#f6f6fa",
    "card_bg": "#ffffff",
    "popover_bg": "#ffffff",
    "dialog_bg": "#fafafb",
    "overview_bg": "#f3f3f5",
    "thumbnail_bg": "#ffffff",
    "active_toggle_bg": "#ffffff",
    "fg": "rgb(0 0 6 / 80%)",
    "shade": "rgb(0 0 6 / 7%)",
    "card_shade": "rgb(0 0 6 / 7%)",
    "headerbar_shade": "rgb(0 0 6 / 12%)",
    "headerbar_darker_shade": "rgb(0 0 6 / 12%)",
    "sidebar_shade": "rgb(0 0 6 / 7%)",
    "sidebar_border": "rgb(0 0 6 / 7%)",
    "secondary_sidebar_shade": "rgb(0 0 6 / 7%)",
    "secondary_sidebar_border": "rgb(0 0 6 / 7%)",
    "popover_shade": "rgb(0 0 6 / 7%)",
    "scrollbar_outline": "#ffffff",
    "border_opacity": 0.15,
    "dim_opacity": 0.55,
    "disabled_opacity": 0.50,
    "destructive": ("#e01b24", "#ffffff", "#c30000"),
    "success": ("#2ec27e", "#ffffff", "#007c3d"),
    "warning": ("#e5a50a", "rgb(0 0 0 / 80%)", "#905400"),
}

DARK: BaseColors = {
    "window_bg": "#222226",
    "view_bg": "#1d1d20",
    "headerbar_bg": "#2e2e32",
    "headerbar_backdrop": "#222226",
    "sidebar_bg": "#2e2e32",
    "sidebar_backdrop": "#28282c",
    "secondary_sidebar_bg": "#28282c",
    "secondary_sidebar_backdrop": "#252529",
    "card_bg": "rgb(255 255 255 / 8%)",
    "popover_bg": "#36363a",
    "dialog_bg": "#36363a",
    "overview_bg": "#28282c",
    "thumbnail_bg": "#39393d",
    "active_toggle_bg": "rgb(255 255 255 / 20%)",
    "fg": "#ffffff",
    "shade": "rgb(0 0 6 / 25%)",
    "card_shade": "rgb(0 0 6 / 36%)",
    "headerbar_shade": "rgb(0 0 6 / 36%)",
    "headerbar_darker_shade": "rgb(0 0 6 / 90%)",
    "sidebar_shade": "rgb(0 0 6 / 25%)",
    "sidebar_border": "rgb(0 0 6 / 36%)",
    "secondary_sidebar_shade": "rgb(0 0 6 / 25%)",
    "secondary_sidebar_border": "rgb(0 0 6 / 36%)",
    "popover_shade": "rgb(0 0 6 / 25%)",
    "scrollbar_outline": "rgb(0 0 6 / 50%)",
    "border_opacity": 0.15,
    "dim_opacity": 0.55,
    "disabled_opacity": 0.50,
    "destructive": ("#c01c28", "#ffffff", "#ff938c"),
    "success": ("#26a269", "#ffffff", "#78e9ab"),
    "warning": ("#cd9309", "rgb(0 0 0 / 80%)", "#ffc252"),
}

#: 高对比度模式覆盖的不透明度字段。
OpacityRole = Literal["border_opacity", "dim_opacity", "disabled_opacity"]

#: libadwaita 高对比度模式。
HIGH_CONTRAST: dict[OpacityRole, float] = {
    "border_opacity": 0.50,
    "dim_opacity": 0.90,
    "disabled_opacity": 0.40,
}

#: 需要转写十六进制的阴影色角色。
ShadeRole = Literal[
    "shade",
    "card_shade",
    "headerbar_shade",
    "headerbar_darker_shade",
    "sidebar_shade",
    "sidebar_border",
    "secondary_sidebar_shade",
    "secondary_sidebar_border",
    "popover_shade",
]

#: 基础表中的状态色角色。
StatusRole = Literal["destructive", "success", "warning"]

_RGB_RE: re.Pattern[str] = re.compile(
    r"rgb\(\s*(\d+)\s+(\d+)\s+(\d+)\s*(?:/\s*([\d.]+)\s*%?\s*)?\)", re.IGNORECASE
)


def parse_color(color: str) -> tuple[int, int, int, float]:
    """解析 ``#rgb``/``#rrggbb``/``#rrggbbaa``/``rgb()``，返回 ``(r, g, b, alpha)``。"""
    color = color.strip()
    if color.startswith("#"):
        value = color[1:]
        if len(value) not in (3, 4, 6, 8) or not re.fullmatch(r"[0-9a-fA-F]+", value):
            raise ValueError(f"无法解析颜色： {color!r}")
        if len(value) in (3, 4):
            value = "".join(ch * 2 for ch in value)
        r, g, b = (int(value[i : i + 2], 16) for i in (0, 2, 4))
        alpha = int(value[6:8], 16) / 255 if len(value) >= 8 else 1.0
        return r, g, b, alpha
    match = _RGB_RE.fullmatch(color)
    if not match:
        raise ValueError(f"无法解析颜色： {color!r}")
    r, g, b = (int(match.group(i)) for i in (1, 2, 3))
    alpha_text = match.group(4)
    if alpha_text is None:
        alpha = 1.0
    elif "%" in color.split("/")[-1]:
        alpha = float(alpha_text) / 100
    else:
        alpha = float(alpha_text)
    if max(r, g, b) > 255 or not 0 <= alpha <= 1:
        raise ValueError(f"颜色分量越界： {color!r}")
    return r, g, b, alpha


def to_hex(r: float, g: float, b: float, alpha: float = 1.0) -> str:
    r, g, b = (min(255, max(0, round(v))) for v in (r, g, b))
    if alpha >= 1.0:
        return f"#{r:02x}{g:02x}{b:02x}"
    return f"#{r:02x}{g:02x}{b:02x}{min(255, max(0, round(alpha * 255))):02x}"


def rgba(color: str, alpha: float) -> str:
    """给颜色强制设置一个 alpha 值。"""
    r, g, b, base_alpha = parse_color(color)
    return to_hex(r, g, b, alpha * base_alpha)


def as_hex(color: str) -> str:
    """把颜色规范化为 ``#rrggbb`` / ``#rrggbbaa``。

    VS Code 主题解析器只接受十六进制，因此 libadwaita 表中的 CSS Color 4 写法
    （如 ``rgb(0 0 6 / 36%)``）必须在写入主题 JSON 之前转换。
    """
    r, g, b, alpha = parse_color(color)
    return to_hex(r, g, b, alpha)


def mix(color_a: str, color_b: str, weight: float) -> str:
    """把 ``weight`` 份的 ``color_a`` 混入 ``color_b``（两者都必须不透明）。"""
    ra, ga, ba, aa = parse_color(color_a)
    rb, gb, bb, ab = parse_color(color_b)
    if aa < 1 or ab < 1:
        raise ValueError("mix() 要求颜色不透明")
    return to_hex(
        ra * weight + rb * (1 - weight),
        ga * weight + gb * (1 - weight),
        ba * weight + bb * (1 - weight),
    )


def over(color: str, bg: str) -> str:
    """把（可能半透明的）颜色合成到不透明背景之上。"""
    r, g, b, alpha = parse_color(color)
    br, bg_, bb, _ = parse_color(bg)
    if alpha >= 1.0:
        return to_hex(r, g, b)
    return to_hex(
        r * alpha + br * (1 - alpha),
        g * alpha + bg_ * (1 - alpha),
        b * alpha + bb * (1 - alpha),
    )


class Palette:
    """某一（模式, 强调色, 对比度）组合的具体颜色。"""

    def __init__(
        self,
        mode: str,
        accent: str = "blue",
        high_contrast: bool = False,
        scheme: dict[str, str | None] | None = None,
    ) -> None:
        if mode not in ("dark", "light"):
            raise ValueError(f"未知主题模式： {mode!r}")
        if accent not in ACCENT_COLORS:
            raise ValueError(f"未知强调色： {accent!r}")
        self.mode: str = mode
        self.accent: str = accent
        self.high_contrast: bool = high_contrast
        self.scheme: dict[str, str | None] = scheme or {}

        base = (DARK if mode == "dark" else LIGHT).copy()
        if high_contrast:
            for key, value in HIGH_CONTRAST.items():
                base[key] = value
        self.base: BaseColors = base
        dark = mode == "dark"

        fg_raw = "rgb(0 0 6 / 100%)" if high_contrast and not dark else base["fg"]
        fg_r, fg_g, fg_b, fg_alpha = parse_color(fg_raw)
        fg = to_hex(fg_r, fg_g, fg_b)
        border_opacity = base["border_opacity"]
        dim_opacity = base["dim_opacity"]
        disabled_opacity = base["disabled_opacity"]

        window = base["window_bg"]
        view = base["view_bg"]
        sidebar = base["sidebar_bg"]
        headerbar = base["headerbar_bg"]

        c: dict[str, str] = {}
        c.update(self._surfaces(base, window))
        self._c: dict[str, str] = c

        # 文本
        c["fg"] = over(fg_raw, window)
        c["fg_window"] = c["fg"]
        c["fg_view"] = over(fg_raw, view)
        c["fg_headerbar"] = over(fg_raw, headerbar)
        c["fg_sidebar"] = over(fg_raw, sidebar)
        c["fg_sidebar_secondary"] = over(fg_raw, base["secondary_sidebar_bg"])
        c["fg_card"] = over(fg_raw, c["bg_card"])
        c["fg_popover"] = over(fg_raw, base["popover_bg"])

        # 交互表面（libadwaita 的按钮/列表行使用 currentColor 的 7-15%）
        for name, bg in (
            ("window", window),
            ("view", view),
            ("sidebar", sidebar),
            ("headerbar", headerbar),
            ("popover", base["popover_bg"]),
            ("card", c["bg_card"]),
        ):
            c[f"bg_hover_{name}"] = over(rgba(fg_raw, 0.07), bg)
            c[f"bg_active_{name}"] = over(rgba(fg_raw, 0.12), bg)
            c[f"fg_dim_{name}"] = over(rgba(fg_raw, dim_opacity), bg)
            c[f"fg_disabled_{name}"] = over(rgba(fg_raw, disabled_opacity), bg)
        c["bg_hover"] = c["bg_hover_window"]
        c["bg_active"] = c["bg_active_window"]
        c["fg_dim"] = c["fg_dim_window"]
        c["fg_disabled"] = c["fg_disabled_window"]
        c["fg_placeholder"] = over(rgba(fg_raw, disabled_opacity), view)
        c["bg_button"] = over(rgba(fg_raw, 0.10), window)
        c["bg_button_headerbar"] = over(rgba(fg_raw, 0.10), headerbar)
        c["bg_button_view"] = over(rgba(fg_raw, 0.10), view)
        c["bg_button_sidebar"] = over(rgba(fg_raw, 0.10), sidebar)
        c["bg_button_hover"] = over(rgba(fg_raw, 0.13), window)
        c["bg_button_active"] = over(rgba(fg_raw, 0.16), window)
        c["bg_input"] = view if not dark else over(rgba(fg_raw, 0.07), view)
        c["bg_input_hover"] = over(rgba(fg_raw, 0.10), c["bg_input"])
        c["bg_input_disabled"] = over(rgba(fg_raw, 0.05), c["bg_input"])

        # 边框
        c["border"] = rgba(fg_raw, border_opacity)
        c["border_strong"] = rgba(fg_raw, max(border_opacity, 0.5))
        c["border_dim"] = rgba(fg_raw, border_opacity * 0.6)
        c["border_input"] = rgba(fg_raw, max(border_opacity, 0.25))
        c["border_input_focus"] = rgba(ACCENT_COLORS[accent][0], 0.5 if not high_contrast else 1.0)
        c["border_tab"] = rgba(fg_raw, border_opacity)
        c["contrast_border"] = fg

        # 阴影色（转为十六进制：VS Code 只接受 #rrggbb / #rrggbbaa）
        shade_keys: tuple[ShadeRole, ...] = (
            "shade",
            "card_shade",
            "headerbar_shade",
            "headerbar_darker_shade",
            "sidebar_shade",
            "sidebar_border",
            "secondary_sidebar_shade",
            "secondary_sidebar_border",
            "popover_shade",
        )
        for name in shade_keys:
            c[name] = as_hex(base[name])
        c["scrollbar_outline"] = as_hex(base["scrollbar_outline"])

        # 滚动条：currentColor 20%（悬停 60%，激活 100%，对应 Adwaita 悬浮滚动条）
        c["scrollbar"] = rgba(fg_raw, 0.40 if self.high_contrast else 0.20)
        c["scrollbar_hover"] = rgba(fg_raw, 0.60)
        c["scrollbar_active"] = fg

        # 强调色
        accent_bg, accent_standalone_light, accent_standalone_dark = ACCENT_COLORS[accent]
        c["accent_bg"] = accent_bg
        # 官方文档表格中九种强调色的前景色均为 #ffffff。
        c["accent_fg"] = "#ffffff"
        c["accent_standalone"] = accent_standalone_dark if dark else accent_standalone_light
        c["accent_hover"] = mix(accent_bg, "#ffffff", 0.15) if dark else mix(accent_bg, "#000000", 0.12)
        c["accent_active"] = mix(accent_bg, "#ffffff", 0.25) if dark else mix(accent_bg, "#000000", 0.20)
        for name, bg in (("view", view), ("window", window), ("sidebar", sidebar)):
            c[f"accent_soft_{name}"] = over(rgba(accent_bg, 0.25), bg)
            c[f"accent_faint_{name}"] = over(rgba(accent_bg, 0.15), bg)
        c["accent_soft"] = c["accent_soft_view"]
        c["accent_faint"] = c["accent_faint_view"]

        # 状态色
        status_keys: tuple[StatusRole, ...] = (
            "destructive",
            "success",
            "warning",
        )
        for name in status_keys:
            bg, fg_status, standalone = base[name]
            c[f"{name}_bg"] = bg
            c[f"{name}_fg"] = over(fg_status, bg)
            c[f"{name}_standalone"] = standalone
        c["error_bg"] = c["destructive_bg"]
        c["error_fg"] = c["destructive_fg"]
        c["error_standalone"] = c["destructive_standalone"]

        # 编辑器
        scheme = self.scheme
        c["editor_bg"] = scheme.get("text_bg") or view
        c["editor_fg"] = scheme.get("text_fg") or c["fg_view"]
        c["editor_line_highlight"] = scheme.get("current_line") or over(rgba(fg_raw, 0.05), view)
        c["editor_line_number"] = scheme.get("line_numbers_fg") or c["fg_dim_view"]
        c["editor_line_number_bg"] = scheme.get("line_numbers_bg") or c["editor_bg"]
        c["editor_cursor"] = scheme.get("cursor") or c["accent_standalone"]
        c["editor_search_match"] = scheme.get("search_match_bg") or c["accent_soft"]
        c["editor_search_match_fg"] = scheme.get("search_match_fg") or c["editor_fg"]
        c["editor_background_pattern"] = scheme.get("background_pattern") or c["editor_bg"]

        # 由强调色推导的编辑器交互色
        c["selection"] = over(rgba(accent_bg, 0.25), c["editor_bg"])
        c["selection_inactive"] = over(rgba(accent_bg, 0.15), c["editor_bg"])
        c["selection_highlight"] = over(rgba(accent_bg, 0.15), c["editor_bg"])
        c["word_highlight"] = over(rgba(accent_bg, 0.20), c["editor_bg"])
        c["word_highlight_strong"] = over(rgba(accent_bg, 0.30), c["editor_bg"])
        c["find_match"] = over(rgba(accent_bg, 0.35), c["editor_bg"])
        c["find_match_highlight"] = over(rgba(accent_bg, 0.20), c["editor_bg"])
        c["bracket_match"] = over(rgba(accent_bg, 0.30), c["editor_bg"])
        c["bracket_border"] = c["accent_standalone"]
        c["indent_guide"] = rgba(fg_raw, max(border_opacity, 0.15))
        c["indent_guide_active"] = rgba(fg_raw, 0.30)
        c["editor_ruler"] = rgba(fg_raw, 0.15)

        # 差异 / 合并
        c["diff_inserted"] = over(rgba(c["success_standalone"], 0.35), c["editor_bg"])
        c["diff_removed"] = over(rgba(c["destructive_standalone"], 0.35), c["editor_bg"])
        c["diff_inserted_bg"] = over(rgba(c["success_standalone"], 0.15), c["editor_bg"])
        c["diff_removed_bg"] = over(rgba(c["destructive_standalone"], 0.15), c["editor_bg"])
        c["diff_base"] = c["editor_bg"]

    def _surfaces(self, base: BaseColors, window: str) -> dict[str, str]:
        return {
            "bg_window": window,
            "bg_view": base["view_bg"],
            "bg_headerbar": base["headerbar_bg"],
            "bg_headerbar_backdrop": base["headerbar_backdrop"],
            "bg_sidebar": base["sidebar_bg"],
            "bg_sidebar_backdrop": base["sidebar_backdrop"],
            "bg_sidebar_secondary": base["secondary_sidebar_bg"],
            "bg_sidebar_secondary_backdrop": base["secondary_sidebar_backdrop"],
            "bg_card": over(base["card_bg"], window),
            "bg_popover": base["popover_bg"],
            "bg_dialog": base["dialog_bg"],
            "bg_overview": base["overview_bg"],
            "bg_thumbnail": base["thumbnail_bg"],
        }

    def __getitem__(self, role: str) -> str:
        try:
            return self._c[role]
        except KeyError:
            raise KeyError(f"未知颜色角色： {role!r}") from None

    def get(self, role: str, default: str | None = None) -> str | None:
        return self._c.get(role, default)

    def over(self, color: str, weight: float, surface: str) -> str:
        """把颜色的 ``weight`` 比例合成到具名表面之上。"""
        return over(rgba(color, weight), self._c[surface])

    def roles(self) -> dict[str, str]:
        return dict(self._c)
