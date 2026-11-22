"""상태별 트레이 아이콘 7종을 assets/icons/*.ico 로 생성한다.

좌표는 16px 격자 기준이고, 각 크기를 8배 슈퍼샘플링으로 따로 그린 뒤 축소한다.
실행: python3 tools/gen_icons.py  (Pillow 8.1 이상)
"""
from pathlib import Path

from PIL import Image, ImageDraw

SIZES = [16, 20, 24, 32, 40, 48]
SS = 8
OUT = Path(__file__).resolve().parent.parent / "assets" / "icons"

LIGHT = (232, 232, 232, 255)
HALO = (24, 24, 24, 150)
INTERIOR = (24, 24, 24, 190)
GREY = (150, 150, 150, 255)
RED = (235, 72, 72, 255)
ORANGE = (245, 166, 35, 255)
GREEN = (60, 196, 106, 255)
BLUE = (78, 168, 255, 255)

BODY = (0.5, 3.0, 13.5, 13.0)
NUB = (13.5, 6.0, 15.5, 10.0)
INNER = (2.5, 5.0, 11.5, 11.0)
STROKE = 1.5


def render(size, painter):
    k = size / 16 * SS
    img = Image.new("RGBA", (size * SS, size * SS), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)

    def box(r):
        return [r[0] * k, r[1] * k, r[2] * k, r[3] * k]

    painter(d, box, k)
    return img.convert("RGBa").resize((size, size), Image.Resampling.BOX).convert("RGBA")


def battery(outline, fill=None, fraction=0.0, extra=None):
    def paint(d, box, k):
        # 밝은 작업 표시줄에서도 보이도록 어두운 테두리를 먼저 깐다
        d.rounded_rectangle(box(_grow(BODY, 0.5)), radius=1.5 * k, outline=HALO, width=round((STROKE + 1) * k))
        d.rectangle(box(_grow(NUB, 0.5)), fill=HALO)
        d.rounded_rectangle(box(BODY), radius=1.2 * k, fill=INTERIOR)
        d.rounded_rectangle(box(BODY), radius=1.2 * k, outline=outline, width=round(STROKE * k))
        d.rectangle(box(NUB), fill=outline)
        if fill and fraction > 0:
            x0, y0, x1, y1 = INNER
            d.rectangle(box((x0, y0, x0 + (x1 - x0) * fraction, y1)), fill=fill)
        if extra:
            extra(d, box, k)

    return paint


def _grow(r, g):
    return (r[0] - g, r[1] - g, r[2] + g, r[3] + g)


def bolt(d, box, k):
    pts = [(8.0, 4.6), (4.2, 8.6), (6.9, 8.6), (5.6, 11.4), (9.8, 7.2), (7.0, 7.2)]
    d.polygon([(x * k, y * k) for x, y in pts], fill=BLUE)


def dots(d, box, k):
    for cx in (4.5, 7.0, 9.5):
        d.ellipse(box((cx - 0.9, 7.1, cx + 0.9, 8.9)), fill=GREY)


def slash(d, box, k):
    d.line([(2.0 * k, 14.5 * k), (14.0 * k, 1.5 * k)], fill=HALO, width=round(2.6 * k))
    d.line([(2.0 * k, 14.5 * k), (14.0 * k, 1.5 * k)], fill=GREY, width=round(1.5 * k))


ICONS = {
    "none": battery(GREY, extra=slash),
    "pending": battery(GREY, extra=dots),
    "wired": battery(LIGHT, extra=bolt),
    "empty": battery(RED),
    "low": battery(LIGHT, ORANGE, 1 / 3),
    "medium": battery(LIGHT, LIGHT, 2 / 3),
    "full": battery(LIGHT, GREEN, 1.0),
}


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for name, painter in ICONS.items():
        images = [render(s, painter) for s in SIZES]
        largest = images[-1]
        largest.save(
            OUT / f"{name}.ico",
            format="ICO",
            sizes=[(s, s) for s in SIZES],
            append_images=images[:-1],
        )
    print(f"{len(ICONS)} icons -> {OUT}")


if __name__ == "__main__":
    main()
