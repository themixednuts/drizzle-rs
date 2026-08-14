"""WCAG contrast check for the Plotter token set.

Converts each oklch() token to sRGB and reports the ratio for every
foreground/surface pair the design actually renders, in both modes.
"""

import math

# -- colour conversion -------------------------------------------------------


def oklch_to_srgb(L, C, h_deg):
    h = math.radians(h_deg)
    a, b = C * math.cos(h), C * math.sin(h)

    l_ = L + 0.3963377774 * a + 0.2158037573 * b
    m_ = L - 0.1055613458 * a - 0.0638541728 * b
    s_ = L - 0.0894841775 * a - 1.2914855480 * b
    l, m, s = l_**3, m_**3, s_**3

    r = +4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s
    g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s
    bl = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s

    def enc(u):
        u = max(0.0, min(1.0, u))
        return 1.055 * u ** (1 / 2.4) - 0.055 if u > 0.0031308 else 12.92 * u

    return tuple(enc(v) for v in (r, g, bl))


def luminance(rgb):
    def lin(u):
        return u / 12.92 if u <= 0.04045 else ((u + 0.055) / 1.055) ** 2.4

    r, g, b = (lin(v) for v in rgb)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def ratio(fg, bg):
    a, b = luminance(fg), luminance(bg)
    hi, lo = max(a, b), min(a, b)
    return (hi + 0.05) / (lo + 0.05)


def hexs(rgb):
    return "#" + "".join(f"{round(v * 255):02X}" for v in rgb)


# -- tokens ------------------------------------------------------------------
# (light, dark) exactly as declared in app.css.

T = {
    "background":           ((0.968, 0.002, 285), (0.145, 0.004, 285)),
    "card":                 ((0.995, 0.001, 285), (0.188, 0.005, 285)),
    "muted":                ((0.935, 0.003, 285), (0.238, 0.006, 285)),
    "surface-inset":        ((0.945, 0.003, 285), (0.118, 0.004, 285)),
    "surface-chrome":       ((0.990, 0.001, 285), (0.128, 0.004, 285)),
    "signal-wash":          ((0.965, 0.030, 122), (0.210, 0.030, 122)),
    "signal-wash-strong":   ((0.930, 0.070, 122), (0.265, 0.055, 122)),
    "caution":              ((0.900, 0.006, 285), (0.260, 0.006, 285)),

    "foreground":           ((0.205, 0.006, 285), (0.935, 0.003, 285)),
    "foreground-secondary": ((0.375, 0.007, 285), (0.810, 0.006, 285)),
    "muted-foreground":     ((0.500, 0.008, 285), (0.680, 0.008, 285)),
    "foreground-faint":     ((0.660, 0.008, 285), (0.500, 0.008, 285)),
    "signal":               ((0.550, 0.150, 128), (0.920, 0.190, 122)),
    "signal-ink":           ((0.500, 0.140, 128), (0.900, 0.190, 122)),
    "positive":             ((0.500, 0.100, 190), (0.820, 0.110, 190)),
    "negative":             ((0.520, 0.190, 15),  (0.740, 0.170, 15)),
    "caution-foreground":   ((0.280, 0.008, 285), (0.920, 0.004, 285)),
    "caution-border":       ((0.580, 0.008, 285), (0.580, 0.008, 285)),
    "border":               ((0.870, 0.004, 285), (0.278, 0.006, 285)),
    "border-soft":          ((0.922, 0.003, 285), (0.222, 0.006, 285)),
    "primary-foreground":   ((0.990, 0.005, 122), (0.190, 0.050, 122)),
    "series-1":             ((0.520, 0.140, 128), (0.920, 0.190, 122)),
    "series-2":             ((0.500, 0.100, 195), (0.820, 0.120, 195)),
    "series-3":             ((0.480, 0.130, 255), (0.780, 0.120, 255)),
    "series-4":             ((0.530, 0.160, 15),  (0.750, 0.150, 15)),
    "series-5":             ((0.550, 0.120, 75),  (0.820, 0.130, 75)),
    "chart-2":              ((0.480, 0.130, 255), (0.780, 0.120, 255)),
    "chart-3":              ((0.500, 0.100, 195), (0.820, 0.120, 195)),
    "chart-4":              ((0.550, 0.120, 75),  (0.820, 0.130, 75)),
    "ring":                 ((0.580, 0.160, 122), (0.920, 0.190, 122)),
}

# fg, bg, minimum, what it is
PAIRS = [
    # Body copy and its ramp, on every surface it lands on.
    ("foreground", "background", 4.5, "body copy on the page"),
    ("foreground", "card", 4.5, "body copy on a panel"),
    ("foreground", "muted", 4.5, "body copy on a muted panel"),
    ("foreground", "surface-inset", 4.5, "body copy in a chart well"),
    ("foreground", "surface-chrome", 4.5, "wordmark on the header"),
    ("foreground", "signal-wash", 4.5, "body copy on a drizzle-rs row"),
    ("foreground", "signal-wash-strong", 4.5, "body copy on a hovered row"),
    ("foreground-secondary", "background", 4.5, "secondary ink on the page"),
    ("foreground-secondary", "card", 4.5, "secondary ink on a panel"),
    ("foreground-secondary", "signal-wash", 4.5, "secondary ink on a drizzle-rs row"),
    ("muted-foreground", "background", 4.5, "labels on the page"),
    ("muted-foreground", "card", 4.5, "labels on a panel"),
    ("muted-foreground", "muted", 4.5, "labels on a muted panel"),
    ("muted-foreground", "surface-chrome", 4.5, "inactive nav on the header"),
    ("muted-foreground", "signal-wash", 4.5, "labels on a drizzle-rs row"),
    ("muted-foreground", "signal-wash-strong", 4.5, "labels on a hovered row"),
    ("muted-foreground", "surface-inset", 4.5, "axis ticks in a chart well"),
    # The signal, wherever it carries a name rather than a mark.
    ("signal-ink", "background", 4.5, "drizzle-rs name on the page"),
    ("signal-ink", "card", 4.5, "drizzle-rs name on a panel"),
    ("signal-ink", "signal-wash", 4.5, "drizzle-rs name on its own row"),
    ("signal-ink", "signal-wash-strong", 4.5, "drizzle-rs name on a hovered row"),
    ("primary-foreground", "signal", 4.5, "text on a filled signal chip"),
    # Status text always carries a sign or a word, so it is text.
    ("positive", "background", 4.5, "a gain on the page"),
    ("positive", "card", 4.5, "a gain on a panel"),
    ("negative", "background", 4.5, "a loss on the page"),
    ("negative", "card", 4.5, "a loss on a panel"),
    ("caution-foreground", "caution", 4.5, "copy in the caution panel"),
    # Marks: 3:1. Plotted lines, bars, rail dots, and the focus ring.
    ("signal", "background", 3.0, "the signal trace on the page"),
    ("signal", "card", 3.0, "the signal trace on a panel"),
    ("signal", "surface-inset", 3.0, "the signal trace in a chart well"),
    ("series-1", "surface-inset", 3.0, "series 1 in a chart well"),
    ("series-2", "surface-inset", 3.0, "series 2 in a chart well"),
    ("series-3", "surface-inset", 3.0, "series 3 in a chart well"),
    ("series-1", "card", 3.0, "series 1 on a panel"),
    ("series-2", "card", 3.0, "series 2 on a panel"),
    ("series-3", "card", 3.0, "series 3 on a panel"),
    ("series-4", "surface-inset", 3.0, "series 4 in a chart well"),
    ("series-5", "surface-inset", 3.0, "series 5 in a chart well"),
    ("series-4", "card", 3.0, "series 4 on a panel"),
    ("series-5", "card", 3.0, "series 5 on a panel"),
    ("chart-2", "surface-inset", 3.0, "latency line in a chart well"),
    ("chart-3", "surface-inset", 3.0, "memory line in a chart well"),
    ("chart-4", "surface-inset", 3.0, "cpu line in a chart well"),
    ("ring", "background", 3.0, "the focus ring on the page"),
    ("ring", "card", 3.0, "the focus ring on a panel"),
    ("caution-border", "caution", 3.0, "the hatch in the caution panel"),
]

modes = ("light", "dark")
fails = []
print(f"{'':<58}{'light':>9}{'dark':>9}")
print("-" * 76)
for fg, bg, need, what in PAIRS:
    row = []
    for i in range(2):
        r = ratio(oklch_to_srgb(*T[fg][i]), oklch_to_srgb(*T[bg][i]))
        row.append(r)
        if r < need:
            fails.append((what, modes[i], fg, bg, r, need))
    mark = lambda r: f"{r:.2f}" + ("" if r >= need else "!")
    print(f"{what:<50}{need:>5.1f}:1{mark(row[0]):>9}{mark(row[1]):>9}")

print()
if fails:
    print(f"{len(fails)} FAILING:")
    for what, mode, fg, bg, r, need in fails:
        print(f"  {mode:<5} {fg} on {bg}: {r:.2f} < {need}  ({what})")
else:
    print("All pairs pass.")

print("\nSwatches (light / dark):")
for name in T:
    print(f"  {name:<22} {hexs(oklch_to_srgb(*T[name][0]))}  {hexs(oklch_to_srgb(*T[name][1]))}")
