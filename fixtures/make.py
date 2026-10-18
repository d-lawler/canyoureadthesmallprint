"""Generate synthetic ad artwork with known ground truth for testing.

python3 fixtures/make.py  ->  fixtures/*.png|jpg + fixtures/truth.json
"""
import json, os, random
from PIL import Image, ImageDraw, ImageFilter, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
REG = "/usr/share/fonts/opentype/inter/Inter-Regular.otf"
BOLD = "/usr/share/fonts/opentype/inter/Inter-Bold.otf"
LEGAL = "Terms apply. 18+ UK residents only. Offer ends 31/12/26. See site for full T&Cs."
LEGAL2 = "Subject to availability and status. Representative 29.9% APR variable."
truth = {}

def font(p, size): return ImageFont.truetype(p, size)

def photo(w, h, seed=1):
    """Blurry blobs: stands in for a photographic background."""
    random.seed(seed)
    im = Image.new("RGB", (w // 8, h // 8))
    d = ImageDraw.Draw(im)
    for _ in range(40):
        x, y = random.randrange(w // 8), random.randrange(h // 8)
        r = random.randrange(5, w // 20)
        c = tuple(random.randrange(30, 220) for _ in range(3))
        d.ellipse([x - r, y - r, x + r, y + r], fill=c)
    return im.filter(ImageFilter.GaussianBlur(6)).resize((w, h), Image.BICUBIC)

def gradient(w, h, top, bottom):
    im = Image.new("RGB", (w, h))
    d = ImageDraw.Draw(im)
    for y in range(h):
        t = y / (h - 1)
        d.line([(0, y), (w, y)], fill=tuple(int(a + (b - a) * t) for a, b in zip(top, bottom)))
    return im

def wrap(d, text, f, max_w):
    words, lines, cur = text.split(), [], ""
    for w in words:
        t = (cur + " " + w).strip()
        if d.textlength(t, font=f) <= max_w: cur = t
        else: lines.append(cur); cur = w
    lines.append(cur)
    return lines

def text_block(im, xy, text, size, fill, max_w, path=REG, lh=1.3):
    d = ImageDraw.Draw(im)
    f = font(path, size)
    x, y = xy
    for ln in wrap(d, text, f, max_w):
        d.text((x, y), ln, font=f, fill=fill)
        y += int(size * lh)
    return y

def save(name, im, dpi, **t):
    p = os.path.join(HERE, name)
    if name.endswith(".jpg"): im.save(p, quality=92, dpi=(dpi, dpi))
    else: im.save(p, dpi=(dpi, dpi))
    truth[name] = t

# 1. Story, white legal on navy: clear AAA
im = Image.new("RGB", (1080, 1920), (16, 28, 64))
text_block(im, (80, 300), "Summer, sorted.", 140, (255, 255, 255), 920, BOLD)
text_block(im, (80, 1700), LEGAL, 30, (255, 255, 255), 920)
save("story_navy.png", im, 72, format="story", font_px=30, level="AAA")

# 2. Story, pale grey legal on white: fails
im = Image.new("RGB", (1080, 1920), (255, 255, 255))
text_block(im, (80, 300), "New season", 140, (20, 20, 20), 920, BOLD)
text_block(im, (80, 1720), LEGAL, 28, (0x9A, 0x9A, 0x9A), 920)
save("story_grey_fail.png", im, 72, format="story", font_px=28, level="Fail")

# 3. MPU exported @2x, #767676 on white: just AA, near threshold
im = Image.new("RGB", (600, 500), (255, 255, 255))
text_block(im, (30, 60), "Save 20%", 80, (200, 0, 40), 540, BOLD)
text_block(im, (30, 400), LEGAL2, 20, (0x76, 0x76, 0x76), 540)
save("mpu_2x.png", im, 144, format="mpu", font_px=20, level="AA")

# 4. Feed portrait, white legal over a photo: varied background
im = photo(1080, 1350, seed=3)
im.paste(gradient(1080, 400, (60, 60, 60), (10, 10, 10)), (0, 950))
text_block(im, (60, 200), "Go further", 120, (255, 255, 255), 960, BOLD)
text_block(im, (60, 1180), LEGAL, 30, (255, 255, 255), 960)
save("feed_photo.jpg", im, 72, format="feed_portrait", font_px=30)

# 5. 48-sheet supplied at 10%, 300dpi (609.6 x 304.8mm)
im = gradient(7200, 3600, (250, 220, 60), (240, 160, 20))
text_block(im, (400, 600), "Fresh every day", 900, (30, 20, 0), 6400, BOLD)
text_block(im, (400, 3200), LEGAL, 110, (60, 40, 0), 6400)
save("48sheet_10pct.png", im, 300, format="48_sheet", font_px=110)

# 6. A4 portrait press ad, 300dpi, 7pt legal (29px)
im = Image.new("RGB", (2480, 3508), (245, 240, 230))
text_block(im, (180, 400), "Quiet luxury.", 260, (20, 20, 20), 2100, BOLD)
text_block(im, (180, 3200), LEGAL2 + " " + LEGAL, 29, (90, 90, 90), 2100)
save("a4_press.jpg", im, 300, format="a4", font_px=29)

# 7. Odd crop: unknown size, should ask one question
im = Image.new("RGB", (1234, 777), (0, 0, 0))
text_block(im, (60, 120), "Limited drop", 110, (255, 255, 255), 1100, BOLD)
text_block(im, (60, 680), LEGAL, 22, (180, 180, 180), 1100)
save("odd_size.png", im, 72, format=None, font_px=22)

# 8. D6-style portrait with dark legal on gradient
im = gradient(1080, 1920, (255, 255, 255), (170, 200, 230))
text_block(im, (80, 300), "Ride the city", 130, (10, 30, 80), 920, BOLD)
text_block(im, (80, 1760), LEGAL, 30, (40, 60, 100), 920)
save("story_gradient.png", im, 72, format="story", font_px=30)

# 9. Feed square, white legal straight over a photo: worst case matters
im = photo(1080, 1080, seed=7)
text_block(im, (60, 160), "Weekend", 150, (255, 255, 255), 960, BOLD)
text_block(im, (60, 930), LEGAL, 28, (255, 255, 255), 960)
save("feed_on_photo.png", im, 72, format="feed_square", font_px=28)

# 10. Two candidate blocks: tiny UI text on a phone screen + the real super
im = photo(1080, 1920, seed=11)
d = ImageDraw.Draw(im)
d.rounded_rectangle([360, 900, 860, 1500], 24, fill=(240, 238, 230))
text_block(im, (390, 960), "Check our family calendar for a 2-hour slot next week when we're both free and suggest date ideas.", 18, (70, 70, 70), 440)
text_block(im, (390, 1120), "Based on your calendar there is one clear 2-hour window during school hours.", 18, (110, 110, 110), 440)
d.rectangle([0, 1640, 1080, 1800], fill=(30, 30, 30))
text_block(im, (300, 1660), "Sequences shortened.", 40, (255, 255, 255), 700)
text_block(im, (230, 1715), "Check responses for accuracy.", 40, (255, 255, 255), 700)
save("story_two_blocks.png", im, 72, format="story", font_px=18)

# 11. Italic white super with a soft drop shadow straight over a photo
ITAL = "/usr/share/fonts/opentype/inter/Inter-Italic.otf"
im = photo(1080, 1920, seed=5)
sh = Image.new("RGBA", im.size, (0, 0, 0, 0))
sd = ImageDraw.Draw(sh)
for (y, t) in [(1560, "Sequences shortened"), (1610, "Check responses for accuracy")]:
    f = font(ITAL, 44); w = sd.textlength(t, font=f)
    sd.text(((1080 - w) / 2 + 2, y + 3), t, font=f, fill=(0, 0, 0, 200))
sh = sh.filter(ImageFilter.GaussianBlur(3))
im = Image.alpha_composite(im.convert("RGBA"), sh).convert("RGB")
d = ImageDraw.Draw(im)
for (y, t) in [(1560, "Sequences shortened"), (1610, "Check responses for accuracy")]:
    f = font(ITAL, 44); w = d.textlength(t, font=f)
    d.text(((1080 - w) / 2, y), t, font=f, fill=(255, 255, 255))
save("story_super_photo.png", im, 72, format="story", font_px=44)

json.dump(truth, open(os.path.join(HERE, "truth.json"), "w"), indent=2)
print("wrote", len(truth), "fixtures")
