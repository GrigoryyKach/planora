"""Subsets the bundled Noto Sans (SIL OFL) to Latin + Cyrillic so the PDF export stays small.

Usage: python tools/subset_fonts.py <full NotoSans-Regular.ttf> <full NotoSans-Bold.ttf>
Writes assets/fonts/NotoSans-Regular.ttf and NotoSans-Bold.ttf (subset). Needs: pip install fonttools
"""
import os, sys
from fontTools import subset

UNICODES = (
    list(range(0x20, 0x7F)) + list(range(0xA0, 0x180)) + list(range(0x400, 0x530))
    + [0x2010, 0x2011, 0x2012, 0x2013, 0x2014, 0x2018, 0x2019, 0x201A, 0x201C, 0x201D, 0x201E, 0x2022, 0x2026, 0x2116, 0x20AC, 0x20B4, 0x20BD, 0x2192]
)
out = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "fonts")
for src in sys.argv[1:3]:
    opts = subset.Options()
    opts.layout_features = ["kern", "liga"]
    opts.name_IDs = ["*"]
    opts.notdef_outline = True
    font = subset.load_font(src, opts)
    s = subset.Subsetter(opts)
    s.populate(unicodes=UNICODES)
    s.subset(font)
    dst = os.path.join(out, os.path.basename(src))
    subset.save_font(font, dst, opts)
    print(dst, os.path.getsize(dst))
