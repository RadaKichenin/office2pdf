# Zero-height custom geometry rendered as a stroked rectangle

The `SALES FORECAST` title rule on page 8 of the #1220 deck is an open
two-point custom geometry inside a zero-height shape box:

```xml
<a:xfrm><a:off x="942535" y="1337304"/><a:ext cx="3708000" cy="0"/></a:xfrm>
<a:custGeom>
  <a:pathLst><a:path w="3218815">
    <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
    <a:lnTo><a:pt x="3218395" y="0"/></a:lnTo>
  </a:path></a:pathLst>
</a:custGeom>
<a:ln w="54863"><a:solidFill><a:schemeClr val="accent1"/></a:solidFill></a:ln>
```

`<a:path>` declares no `h`, so its vertical coordinate space inherited the
shape's own `cy="0"`. The geometry parser dropped every subpath whose
coordinate space had a zero axis, which left the caller's rectangle fallback
to stroke a closed zero-height box. `a:ln` carries no `<a:miter>` or
`<a:bevel>`, so DrawingML's default round join applied — and on a box with no
height the two corner joins are the whole of each end, printing as
semicircles.

## Measured

`mutool draw -F trace` of page 8, stroke of the title rule:

| | path | ends |
| --- | --- | --- |
| native PowerPoint 16.112 | `moveto (0,0)` `lineto (3707516,0)` | flat |
| LibreOffice Impress 26.2.5.2 | `moveto (74.183,434.721)` `lineto (366.094,434.721)` | flat, `linecap="0,0,0"` |
| office2pdf before | `moveto (0,0)` `lineto (291.9685,0)` `lineto (0,0)` `closepath` | round joins |
| office2pdf after | `moveto (0,0)` `lineto (291.9304,0)` | flat |

Both references draw one open segment; only the closed fallback produced the
round ends. The rule's length follows the same fix: the path's own
`3218395 / 3218815` of the `3708000` EMU box is 291.9304pt, which is exactly
the `3707516` EMU the native export draws, where the rectangle fallback spent
the whole 291.9685pt box.

## Fix

`SubpathBuilder::start_subpath` normalizes each axis independently. A
zero-length axis holds one position, so every coordinate on it is 0 while the
other axis still scales. A coordinate space with neither axis still yields
nothing, so the rectangle fallback is unchanged for a shape with no box at
all.
