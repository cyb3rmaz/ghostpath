import os
from PIL import Image, ImageDraw

def create_icon(size):
    img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    
    pad = int(size * 0.08)
    # Dark rounded background
    draw.rounded_rectangle([pad, pad, size - pad, size - pad], radius=int(size * 0.22), fill=(15, 23, 20, 255))
    # Border
    draw.rounded_rectangle([pad, pad, size - pad, size - pad], radius=int(size * 0.22), outline=(34, 197, 94, 255), width=max(1, int(size * 0.04)))
    
    # Shield / Ghost silhouette in center
    cx, cy = size // 2, size // 2
    r = int(size * 0.28)
    
    # Outer circle / node glow
    draw.ellipse([cx - r, cy - r, cx + r, cy + r], outline=(74, 222, 128, 200), width=max(1, int(size * 0.03)))
    
    # Inner node
    r_in = max(2, int(size * 0.12))
    draw.ellipse([cx - r_in, cy - r_in, cx + r_in, cy + r_in], fill=(34, 197, 94, 255))
    
    # Connection lines / ticks
    w_line = max(1, int(size * 0.03))
    draw.line([cx, pad + int(size * 0.12), cx, cy - r_in], fill=(34, 197, 94, 220), width=w_line)
    draw.line([cx, cy + r_in, cx, size - pad - int(size * 0.12)], fill=(34, 197, 94, 220), width=w_line)
    draw.line([pad + int(size * 0.12), cy, cx - r_in, cy], fill=(34, 197, 94, 220), width=w_line)
    draw.line([cx + r_in, cy, size - pad - int(size * 0.12), cy], fill=(34, 197, 94, 220), width=w_line)
    
    return img

def main():
    gui_icons = "crates/ghostpath-gui/icons"
    pkg_icons = "packaging/icons"
    os.makedirs(gui_icons, exist_ok=True)
    os.makedirs(pkg_icons, exist_ok=True)
    
    sizes = [16, 24, 32, 48, 64, 128, 256, 512]
    images = {}
    for s in sizes:
        img = create_icon(s)
        images[s] = img
        img.save(os.path.join(pkg_icons, f"{s}x{s}.png"))
        
    # GUI icons
    images[32].save(os.path.join(gui_icons, "32x32.png"))
    images[128].save(os.path.join(gui_icons, "128x128.png"))
    images[256].save(os.path.join(gui_icons, "128x128@2x.png"))
    images[512].save(os.path.join(gui_icons, "icon.png"))
    images[256].save(os.path.join(gui_icons, "icon.ico"), format='ICO', sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    
    # Also write packaging/ghostpath.svg
    svg_content = '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <rect width="440" height="440" x="36" y="36" rx="100" fill="#0F1714" stroke="#22C55E" stroke-width="18" />
  <circle cx="256" cy="256" r="140" fill="none" stroke="#4ADE80" stroke-width="14" stroke-dasharray="16 12" />
  <circle cx="256" cy="256" r="60" fill="#22C55E" />
  <line x1="256" y1="80" x2="256" y2="196" stroke="#22C55E" stroke-width="14" stroke-linecap="round" />
  <line x1="256" y1="316" x2="256" y2="432" stroke="#22C55E" stroke-width="14" stroke-linecap="round" />
  <line x1="80" y1="256" x2="196" y2="256" stroke="#22C55E" stroke-width="14" stroke-linecap="round" />
  <line x1="316" y1="256" x2="432" y2="256" stroke="#22C55E" stroke-width="14" stroke-linecap="round" />
</svg>'''
    with open("packaging/ghostpath.svg", "w", encoding="utf-8") as f:
        f.write(svg_content)
    with open(os.path.join(gui_icons, "icon.svg"), "w", encoding="utf-8") as f:
        f.write(svg_content)
    print("Icons generated successfully.")

if __name__ == "__main__":
    main()
