"""Generate deterministic synthetic MinerU V1 transport/normalization fixtures.

Run with Python 3 from any directory. These are contract fixtures, not captured
MinerU responses and not evidence of OCR accuracy. Images come from the checked
in Office inputs; no network, credentials, or document conversion is required.
"""

import json
from pathlib import Path
from zipfile import ZIP_STORED, ZipFile, ZipInfo


ROOT = Path(__file__).resolve().parent
TABLE = "<table><tr><th>Item</th><th>Revenue</th></tr><tr><td>Alpha</td><td>120</td></tr><tr><td>Beta</td><td>240</td></tr></table>"


def generate(name):
    suffix = name.rsplit(".", 1)[1]
    source = "synthetic.xlsx" if suffix == "xlsx" else "synthetic.docx"
    image_path = "xl/media/image1.png" if suffix == "xlsx" else "word/media/image1.png"
    with ZipFile(ROOT / source) as original:
        image = original.read(image_path)
    middle = {
        "schema": "docvortex.middle",
        "schema_version": "2.0",
        "metadata": {"file_suffix": suffix, "filename": name},
        "extensions": {"mineru": {"tier": "standard" if suffix == "pdf" else "flash"}},
        "pages": [{"page_idx": 0, "blocks": [
            {"type": "text", "index": 0, "content": "Synthetic Alpha revenue 120; Beta revenue 240."},
            {"type": "table", "index": 1, "content": [{"type": "table_body", "index": 5, "content": TABLE}]},
            {"type": "image", "index": 2, "content": [
                {"type": "image_body", "index": 3, "image_path": "images/figure.png", "content": ""},
                {"type": "image_caption", "index": 4, "content": "Synthetic Alpha revenue chart"},
            ]},
        ]}],
    }
    files = {
        "middle_json.json": json.dumps(middle, ensure_ascii=False, sort_keys=True).encode(),
        "structured_content.json": json.dumps({"synthetic": True, "pages": middle["pages"]}, sort_keys=True).encode(),
        "markdown.md": f"# Synthetic fixture: {name}\n\nAlpha revenue 120; Beta revenue 240.\n\n{TABLE}\n\n![Synthetic chart](images/figure.png)\n".encode(),
        "images/figure.png": image,
    }
    with ZipFile(ROOT / f"{name}.zip", "w") as archive:
        for path, data in files.items():
            entry = ZipInfo(path, date_time=(2020, 1, 1, 0, 0, 0))
            entry.compress_type = ZIP_STORED
            entry.create_system = 3
            entry.external_attr = 0o100644 << 16
            archive.writestr(entry, data)


if __name__ == "__main__":
    for filename in ("synthetic.docx", "synthetic.xlsx", "synthetic.pdf", "synthetic-scanned.pdf"):
        generate(filename)
