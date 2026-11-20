"""Builds the documentation website (GitHub Pages) from docs/site/content.

    py -I docs/site/build.py                 write the site to target/docs-site
    py -I docs/site/build.py --out DIR       somewhere else
    py -I docs/site/build.py --check         build and fail on any problem (CI)

Only the Python standard library. Every language has its own folder in
docs/site/content/<code>/ with one Markdown file per page (the names in
PAGES below) and _site.json with the words of the frame (menu, footer).
English is the reference, the other languages are translations of it.

The Markdown is a small subset, enough for these pages:

    # Page title            (first line, once)
    ## Section              ## Section {#anchor} for a fixed anchor
    ### Subsection
    - list item             1. numbered item (continuation lines indented)
    > note                  shown as a highlighted box
    | table | rows |        the second row is the |---| separator
    ```                     code block
    ![alt](screenshot-faults.png)   on its own line: a screenshot from docs/
    `code`  **bold**  *italic*  [text](faults.md)  [text](https://...)

Links to other pages are written as <page>.md and become .html; the build
checks that they exist. Screenshots come from docs/ (German ones from
docs/de/ for the German pages) and are copied into the site.

Checks (always, --check turns warnings into a failure): every page exists
in every language, every page starts with its title, links and images
resolve, no long dashes (U+2014, U+2013), no placeholders left over.
"""

from __future__ import annotations

import datetime
import html
import json
import os
import re
import shutil
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
CONTENT = os.path.join(HERE, "content")
DOCS = os.path.join(ROOT, "docs")
sys.path.insert(0, os.path.join(ROOT, "tools"))

from readme_languages import LANGUAGES  # noqa: E402  (same list as the READMEs)

REPO = "https://github.com/SenseiIssei/Caruspex"
BASE_URL = "https://senseiissei.github.io/Caruspex/"
RTL = {"ar"}

# The menu: (group key in _site.json, [pages]). The order is the reading order.
GROUPS = [
    ("group_start", ["index", "install", "first-steps", "connect"]),
    (
        "group_interfaces",
        [
            "interfaces",
            "elm327-usb",
            "elm327-bluetooth",
            "elm327-wifi",
            "obdlink-stn",
            "kkl",
            "kdcan",
            "enet",
            "j2534",
            "slcan",
            "socketcan",
            "pcan",
            "kvaser",
            "doip",
        ],
    ),
    ("group_using", ["faults", "live", "xray", "used-car", "tuning-simulator"]),
    ("group_more", ["privacy", "security", "legal", "faq"]),
]
PAGES = [page for _, pages in GROUPS for page in pages]

SITE_KEYS = [
    "language_name",
    "site_title",
    "tagline",
    "group_start",
    "group_interfaces",
    "group_using",
    "group_more",
    "menu",
    "language",
    "skip",
    "download",
    "source",
    "previous",
    "next",
    "improve",
    "footer",
]

LONG_DASHES = (chr(0x2014), chr(0x2013))


class Problems:
    def __init__(self) -> None:
        self.items: list[str] = []

    def add(self, where: str, message: str) -> None:
        self.items.append(f"{where}: {message}")


# --- Markdown ------------------------------------------------------------------------------

INLINE = re.compile(
    r"(`[^`]+`)"  # code
    r"|(\*\*[^*]+\*\*)"  # bold
    r"|(\*[^*\s][^*]*\*)"  # italic
    r"|(\[[^\]]+\]\([^)\s]+\))"  # link
)
LINK = re.compile(r"\[([^\]]+)\]\(([^)\s]+)\)")
IMAGE = re.compile(r"^!\[([^\]]*)\]\(([^)\s]+)\)$")
HEADING = re.compile(r"^(#{1,3})\s+(.*?)(?:\s+\{#([a-z0-9-]+)\})?\s*$")
ORDERED = re.compile(r"^\d+\.\s+")


class Page:
    def __init__(self, lang: str, slug: str, problems: Problems) -> None:
        self.lang = lang
        self.slug = slug
        self.problems = problems
        self.where = f"content/{lang}/{slug}.md"
        self.title = ""
        self.links: list[tuple[str, str]] = []  # (page, anchor)
        self.anchors: set[str] = set()
        self.images: list[str] = []
        self.sections: list[tuple[str, str]] = []  # (anchor, text) of ## headings

    # Inline text: escape, then code, bold, italic and links.
    def inline(self, text: str) -> str:
        out = []
        pos = 0
        for m in INLINE.finditer(text):
            out.append(html.escape(text[pos : m.start()], quote=False))
            token = m.group(0)
            if m.group(1):
                out.append("<code>" + html.escape(token[1:-1], quote=False) + "</code>")
            elif m.group(2):
                out.append("<strong>" + self.inline(token[2:-2]) + "</strong>")
            elif m.group(3):
                out.append("<em>" + self.inline(token[1:-1]) + "</em>")
            else:
                link = LINK.match(token)
                assert link
                out.append(f'<a href="{html.escape(self.href(link.group(2)))}">{self.inline(link.group(1))}</a>')
            pos = m.end()
        out.append(html.escape(text[pos:], quote=False))
        return "".join(out)

    def href(self, target: str) -> str:
        if re.match(r"^(https?:|mailto:)", target):
            return target
        if target.startswith("#"):
            self.links.append((self.slug, target[1:]))
            return target
        page, _, anchor = target.partition("#")
        if not page.endswith(".md"):
            self.problems.add(self.where, f"link to '{target}': other pages are linked as <page>.md")
            return target
        page = page[:-3]
        self.links.append((page, anchor))
        return page + ".html" + ("#" + anchor if anchor else "")

    def image(self, alt: str, name: str) -> str:
        self.images.append(name)
        local = os.path.join(DOCS, self.lang, name)
        src = f"../img/{self.lang}/{name}" if os.path.isfile(local) else f"../img/{name}"
        return (
            f'<figure><img src="{html.escape(src)}" alt="{html.escape(alt)}" loading="lazy">'
            f"<figcaption>{self.inline(alt)}</figcaption></figure>"
        )

    def anchor_for(self, text: str, explicit: str | None) -> str:
        if explicit:
            anchor = explicit
        else:
            anchor = re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-")
            if not anchor or not anchor.isascii():
                anchor = f"s{len(self.anchors) + 1}"
        base, n = anchor, 2
        while anchor in self.anchors:
            anchor = f"{base}-{n}"
            n += 1
        self.anchors.add(anchor)
        return anchor

    def render(self, text: str) -> str:
        lines = text.replace("\r\n", "\n").split("\n")
        out: list[str] = []
        i = 0
        paragraph: list[str] = []

        def flush() -> None:
            if paragraph:
                out.append("<p>" + self.inline(" ".join(s.strip() for s in paragraph)) + "</p>")
                paragraph.clear()

        while i < len(lines):
            line = lines[i]
            stripped = line.strip()
            if not stripped:
                flush()
                i += 1
                continue
            if stripped.startswith("```"):
                flush()
                block = []
                i += 1
                while i < len(lines) and not lines[i].strip().startswith("```"):
                    block.append(lines[i])
                    i += 1
                if i >= len(lines):
                    self.problems.add(self.where, "code block is not closed")
                out.append('<pre><code dir="ltr">' + html.escape("\n".join(block), quote=False) + "</code></pre>")
                i += 1
                continue
            heading = HEADING.match(stripped)
            if heading and not line.startswith(" "):
                flush()
                level = len(heading.group(1))
                content = heading.group(2)
                if level == 1:
                    if self.title:
                        self.problems.add(self.where, "more than one '# ' title")
                    self.title = content
                    out.append(f"<h1>{self.inline(content)}</h1>")
                else:
                    anchor = self.anchor_for(content, heading.group(3))
                    if level == 2:
                        self.sections.append((anchor, content))
                    out.append(
                        f'<h{level} id="{anchor}">{self.inline(content)}'
                        f'<a class="anchor" href="#{anchor}" aria-hidden="true">#</a></h{level}>'
                    )
                i += 1
                continue
            image = IMAGE.match(stripped)
            if image:
                flush()
                out.append(self.image(image.group(1), image.group(2)))
                i += 1
                continue
            if stripped.startswith(">"):
                flush()
                block = []
                while i < len(lines) and lines[i].strip().startswith(">"):
                    block.append(lines[i].strip()[1:].strip())
                    i += 1
                out.append('<div class="note">' + self.render("\n".join(block)) + "</div>")
                continue
            if stripped.startswith("|"):
                flush()
                rows = []
                while i < len(lines) and lines[i].strip().startswith("|"):
                    rows.append([c.strip() for c in lines[i].strip().strip("|").split("|")])
                    i += 1
                if len(rows) < 2 or not all(re.fullmatch(r":?-+:?", c) for c in rows[1]):
                    self.problems.add(self.where, "a table needs a |---| line below its head")
                    continue
                head = "".join(f"<th>{self.inline(c)}</th>" for c in rows[0])
                body = "".join(
                    "<tr>" + "".join(f"<td>{self.inline(c)}</td>" for c in row) + "</tr>" for row in rows[2:]
                )
                out.append(f'<div class="table"><table><thead><tr>{head}</tr></thead><tbody>{body}</tbody></table></div>')
                continue
            if stripped.startswith("- ") or ORDERED.match(stripped):
                flush()
                ordered = not stripped.startswith("- ")
                items: list[str] = []
                while i < len(lines):
                    current = lines[i]
                    s = current.strip()
                    starts = s.startswith("- ") if not ordered else bool(ORDERED.match(s))
                    if starts and not current.startswith(" "):
                        items.append(s[2:] if not ordered else ORDERED.sub("", s, count=1))
                    elif s and current.startswith("  ") and items:
                        items[-1] += " " + s
                    else:
                        break
                    i += 1
                tag = "ol" if ordered else "ul"
                out.append(f"<{tag}>" + "".join(f"<li>{self.inline(item)}</li>" for item in items) + f"</{tag}>")
                continue
            paragraph.append(line)
            i += 1
        flush()
        return "\n".join(out)


# --- Frame ---------------------------------------------------------------------------------

STYLE = os.path.join(HERE, "style.css")
TEMPLATE = os.path.join(HERE, "template.html")


def load_site(lang: str, problems: Problems) -> dict[str, str]:
    path = os.path.join(CONTENT, lang, "_site.json")
    if not os.path.isfile(path):
        problems.add(f"content/{lang}", "_site.json is missing")
        return {}
    data = json.load(open(path, encoding="utf-8"))
    for key in SITE_KEYS:
        if not isinstance(data.get(key), str) or not data[key].strip():
            problems.add(f"content/{lang}/_site.json", f"'{key}' is missing")
    for key in data:
        if key not in SITE_KEYS:
            problems.add(f"content/{lang}/_site.json", f"unknown key '{key}'")
    return data


def nav_html(lang: str, site: dict[str, str], titles: dict[str, str], current: str) -> str:
    parts = []
    for group, pages in GROUPS:
        items = []
        for page in pages:
            here = ' aria-current="page"' if page == current else ""
            items.append(f'<li><a href="{page}.html"{here}>{html.escape(titles.get(page, page))}</a></li>')
        parts.append(f'<h2>{html.escape(site.get(group, group))}</h2><ul>{"".join(items)}</ul>')
    return "".join(parts)


def languages_html(current_lang: str, slug: str) -> str:
    items = []
    for code, name in LANGUAGES:
        here = ' aria-current="true"' if code == current_lang else ""
        items.append(f'<li><a href="../{code}/{slug}.html" lang="{code}" hreflang="{code}"{here}>{html.escape(name)}</a></li>')
    return "<ul>" + "".join(items) + "</ul>"


def page_html(template: str, lang: str, slug: str, site: dict[str, str], titles: dict[str, str], body: str) -> str:
    index = PAGES.index(slug)
    prev_link = next_link = ""
    if index > 0:
        p = PAGES[index - 1]
        prev_link = f'<a class="prev" href="{p}.html"><span>{html.escape(site["previous"])}</span>{html.escape(titles[p])}</a>'
    if index + 1 < len(PAGES):
        n = PAGES[index + 1]
        next_link = f'<a class="next" href="{n}.html"><span>{html.escape(site["next"])}</span>{html.escape(titles[n])}</a>'
    alternates = "\n".join(
        f'<link rel="alternate" hreflang="{code}" href="{BASE_URL}{code}/{slug}.html">' for code, _ in LANGUAGES
    )
    full_title = f"{titles[slug]} | {site['site_title']}" if slug != "index" else site["site_title"]
    values = {
        "lang": lang,
        "dir": "rtl" if lang in RTL else "ltr",
        "title": html.escape(full_title),
        "description": html.escape(site["tagline"]),
        "canonical": f"{BASE_URL}{lang}/{slug}.html",
        "alternates": alternates,
        "site_title": html.escape(site["site_title"]),
        "skip": html.escape(site["skip"]),
        "menu": html.escape(site["menu"]),
        "language": html.escape(site["language"]),
        "language_name": html.escape(site["language_name"]),
        "languages": languages_html(lang, slug),
        "nav": nav_html(lang, site, titles, slug),
        "body": body,
        "prev": prev_link,
        "next": next_link,
        "improve": html.escape(site["improve"]),
        "edit_url": f"{REPO}/edit/main/docs/site/content/{lang}/{slug}.md",
        "download": html.escape(site["download"]),
        "download_url": f"{REPO}/releases/latest",
        "source": html.escape(site["source"]),
        "repo": REPO,
        "footer": html.escape(site["footer"]),
    }
    out = template
    for key, value in values.items():
        out = out.replace("{{" + key + "}}", value)
    return out


def root_index(base: str = "", redirect: bool = True) -> str:
    """The entry page: picks the language of the browser, else lists them.
    The 404 page uses it with absolute paths and without the redirect."""
    links = "".join(
        f'<li><a href="{base}{code}/index.html" lang="{code}" hreflang="{code}">{html.escape(name)}</a></li>'
        for code, name in LANGUAGES
    )
    codes = json.dumps([code for code, _ in LANGUAGES])
    script = f"""<script>
(function () {{
  var codes = {codes};
  var wanted = (navigator.languages || [navigator.language || "en"]);
  for (var i = 0; i < wanted.length; i++) {{
    var code = String(wanted[i]).toLowerCase().split("-")[0];
    if (codes.indexOf(code) >= 0) {{ location.replace(code + "/index.html"); return; }}
  }}
  location.replace("en/index.html");
}})();
</script>
""" if redirect else ""
    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Caruspex</title>
<link rel="icon" href="{base}img/logo.png">
<link rel="stylesheet" href="{base}style.css">
{script}</head>
<body class="chooser">
<main>
<img src="{base}img/logo.png" alt="" width="96" height="96">
<h1>Caruspex</h1>
<ul class="language-list">{links}</ul>
</main>
</body>
</html>
"""


def security_txt(now: datetime.datetime) -> str:
    """RFC 9116. The project site lives below /Caruspex/ on github.io, not at
    the root of the origin where the RFC wants the file, so scanners that
    only look at the root miss it; it still names the contact and the policy
    for anyone who looks here. Expires is half a year after the build (the
    RFC asks for less than a year); pages.yml rebuilds the site every month."""
    expires = (now + datetime.timedelta(days=183)).strftime("%Y-%m-%dT00:00:00Z")
    return (
        f"Contact: {REPO}/security/advisories/new\n"
        f"Expires: {expires}\n"
        f"Policy: {REPO}/blob/main/SECURITY.md\n"
        "Preferred-Languages: en, de\n"
        f"Canonical: {BASE_URL}.well-known/security.txt\n"
    )


# --- Build ---------------------------------------------------------------------------------


def text_problems(where: str, text: str, problems: Problems) -> None:
    for dash in LONG_DASHES:
        if dash in text:
            problems.add(where, f"long dash U+{ord(dash):04X}, use a comma, a colon or a hyphen")
    for word in ("TODO", "FIXME", "XXX"):
        if re.search(rf"\b{word}\b", text):
            problems.add(where, f"placeholder '{word}' left over")


def build(out: str) -> Problems:
    problems = Problems()
    template = open(TEMPLATE, encoding="utf-8").read()
    if os.path.isdir(out):
        shutil.rmtree(out)
    os.makedirs(out)

    expected = {f"{slug}.md" for slug in PAGES} | {"_site.json"}
    for code, _ in LANGUAGES:
        folder = os.path.join(CONTENT, code)
        if not os.path.isdir(folder):
            problems.add(f"content/{code}", "the language is missing")
            continue
        for name in sorted(set(os.listdir(folder)) - expected):
            problems.add(f"content/{code}/{name}", "not a page of the site (see PAGES in build.py)")

    for code, _ in LANGUAGES:
        folder = os.path.join(CONTENT, code)
        if not os.path.isdir(folder):
            continue
        site = load_site(code, problems)
        text_problems(f"content/{code}/_site.json", json.dumps(site, ensure_ascii=False), problems)
        rendered: dict[str, Page] = {}
        bodies: dict[str, str] = {}
        for slug in PAGES:
            path = os.path.join(folder, f"{slug}.md")
            if not os.path.isfile(path):
                problems.add(f"content/{code}/{slug}.md", "the page is missing")
                continue
            text = open(path, encoding="utf-8").read()
            page = Page(code, slug, problems)
            if not text.startswith("# "):
                problems.add(page.where, "must start with '# ' and the page title")
            text_problems(page.where, text, problems)
            bodies[slug] = page.render(text)
            rendered[slug] = page
        if len(rendered) != len(PAGES) or any(not site.get(k) for k in SITE_KEYS):
            continue
        titles = {slug: page.title for slug, page in rendered.items()}
        for slug, page in rendered.items():
            for target, anchor in page.links:
                if target not in rendered:
                    problems.add(page.where, f"link to the unknown page '{target}.md'")
                elif anchor and anchor not in rendered[target].anchors:
                    problems.add(page.where, f"link to '{target}.md#{anchor}', the anchor does not exist")
            for image in page.images:
                if not os.path.isfile(os.path.join(DOCS, image)):
                    problems.add(page.where, f"image '{image}' is not in docs/")
        os.makedirs(os.path.join(out, code))
        for slug in PAGES:
            with open(os.path.join(out, code, f"{slug}.html"), "w", encoding="utf-8", newline="\n") as f:
                f.write(page_html(template, code, slug, site, titles, bodies[slug]))

    # Screenshots (English in img/, other languages in img/<code>/ where docs/<code>/ has them).
    os.makedirs(os.path.join(out, "img"))
    for name in sorted(os.listdir(DOCS)):
        source = os.path.join(DOCS, name)
        if name.endswith(".png") and os.path.isfile(source):
            shutil.copyfile(source, os.path.join(out, "img", name))
        elif os.path.isdir(source) and name in dict(LANGUAGES):
            os.makedirs(os.path.join(out, "img", name))
            for image in sorted(os.listdir(source)):
                if image.endswith(".png"):
                    shutil.copyfile(os.path.join(source, image), os.path.join(out, "img", name, image))
    shutil.copyfile(os.path.join(ROOT, "resources", "icons", "caruspex.png"), os.path.join(out, "img", "logo.png"))
    shutil.copyfile(STYLE, os.path.join(out, "style.css"))
    with open(os.path.join(out, "index.html"), "w", encoding="utf-8", newline="\n") as f:
        f.write(root_index())
    with open(os.path.join(out, "404.html"), "w", encoding="utf-8", newline="\n") as f:
        f.write(root_index("/Caruspex/", redirect=False))
    # No Jekyll on GitHub Pages: the files are served as they are.
    open(os.path.join(out, ".nojekyll"), "w").close()
    os.makedirs(os.path.join(out, ".well-known"))
    with open(os.path.join(out, ".well-known", "security.txt"), "w", encoding="utf-8", newline="\n") as f:
        f.write(security_txt(datetime.datetime.now(datetime.timezone.utc)))
    return problems


def main() -> int:
    arguments = sys.argv[1:]
    check = "--check" in arguments
    out = os.path.join(ROOT, "target", "docs-site")
    if "--out" in arguments:
        out = os.path.abspath(arguments[arguments.index("--out") + 1])
    problems = build(out)
    for item in problems.items:
        print(item, file=sys.stderr)
    files = sum(len(names) for _, _, names in os.walk(out))
    if problems.items:
        print(f"{len(problems.items)} problem(s) in the documentation", file=sys.stderr)
        if check:
            return 1
    print(f"documentation written to {out} ({files} files, {len(LANGUAGES)} languages, {len(PAGES)} pages each)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
