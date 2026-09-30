#!/usr/bin/env python3
"""Builds data/2026_cl_predictions.csv from the pundit prediction page.

Source: https://www.ohtashp.com/topics/baseball_yosou/  (2026 pre-season Central League
predictions by commentators, compiled by ohtashp.com). Give the saved markdown copy of the
page as the only argument:

    python3 tools/predictions/build_predictions.py path/to/baseball_yosou.md

Rules
- One prediction per person: the first row in the table. (Where the page marks several
  versions with circled numbers, the first row is also the one marked 1.)
- Skipped: the header, "last year's standings", "preseason game standings", and rows that are
  not a complete 1st-to-6th order (a joke entry and a "not announced" entry).
- Username: romaji, family name first ("Iwase Hitoki"). If that is longer than 16 bytes
  (the on-chain limit) the given name is cut to an initial ("Ogasawara M.").
- Output team numbers follow the Solana program:
  0=Yomiuri, 1=Hanshin, 2=DeNA, 3=Hiroshima, 4=Chunichi, 5=Yakult.
"""

import csv
import re
import sys
import unicodedata
from pathlib import Path

MAX_USERNAME_BYTES = 16  # MAX_USERNAME_LEN in the program

TEAM_INDEX = {"巨人": 0, "阪神": 1, "DeNA": 2, "広島": 3, "中日": 4, "ヤクルト": 5}

# Japanese name (as written on the page) -> (family, given) in Hepburn romaji without macrons,
# or a finished string for names that are not "family given".
ROMAJI = {
    "岩瀬仁紀": ("Iwase", "Hitoki"),
    "里崎智也": ("Satozaki", "Tomoya"),
    "AERA": "AERA",  # a magazine, not a person
    "高木豊": ("Takagi", "Yutaka"),
    "星野伸之": ("Hoshino", "Nobuyuki"),
    "藤本博史": ("Fujimoto", "Hiroshi"),
    "池田親興": ("Ikeda", "Chikaoki"),
    "デーブ大久保": "Dave Okubo",
    "野村弘樹": ("Nomura", "Hiroki"),
    "G.G.佐藤": "G.G. Sato",
    "天谷宗一郎": ("Amaya", "Soichiro"),
    "佐々岡真司": ("Sasaoka", "Shinji"),
    "大野豊": ("Ono", "Yutaka"),
    "緒方孝市": ("Ogata", "Koichi"),
    "安倍友裕": ("Abe", "Tomohiro"),
    "中田廉": ("Nakata", "Ren"),
    "井手正太郎": ("Ide", "Shotaro"),
    "祖父江大輔": ("Sobue", "Daisuke"),
    "阿波野秀幸": ("Awano", "Hideyuki"),
    "矢野燿大": ("Yano", "Akihiro"),
    "中田翔": ("Nakata", "Sho"),
    "亀山つとむ": ("Kameyama", "Tsutomu"),
    "森繁和": ("Mori", "Shigekazu"),
    "伊原春樹": ("Ihara", "Haruki"),
    "野口寿浩": ("Noguchi", "Toshihiro"),
    "小笠原道大": ("Ogasawara", "Michihiro"),
    "片岡篤史": ("Kataoka", "Atsushi"),
    "江本孟紀": ("Emoto", "Takenori"),
    "関本四十四": ("Sekimoto", "Shijushi"),  # reading not verified
    "広澤克実": ("Hirosawa", "Katsumi"),
    "大澤謙一郎": ("Osawa", "Kenichiro"),  # reading not verified
    "関本賢太郎": ("Sekimoto", "Kentaro"),
    "能見篤史": ("Nomi", "Atsushi"),
    "原口文仁": ("Haraguchi", "Fumihito"),
    "野村謙二郎": ("Nomura", "Kenjiro"),
    "T-岡田": "T-Okada",
    "赤星憲広": ("Akahoshi", "Norihiro"),
    "松田宣浩": ("Matsuda", "Nobuhiro"),
    "岸川勝也": ("Kishikawa", "Katsuya"),
    "杉本正": ("Sugimoto", "Tadashi"),
    "若菜嘉晴": ("Wakana", "Yoshiharu"),
    "張本勲": ("Harimoto", "Isao"),
    "有藤通世": ("Arito", "Michiyo"),
    "田淵幸一": ("Tabuchi", "Koichi"),
    "東尾修": ("Higashio", "Osamu"),
    "中畑清": ("Nakahata", "Kiyoshi"),
    "牛島和彦": ("Ushijima", "Kazuhiko"),
    "槙原寛己": ("Makihara", "Hiromi"),
    "伊東勤": ("Ito", "Tsutomu"),
    "辻発彦": ("Tsuji", "Hatsuhiko"),
    "福留孝介": ("Fukudome", "Kosuke"),
    "川又米利": ("Kawamata", "Yonetoshi"),  # reading not verified
    "彦野利勝": ("Hikono", "Toshikatsu"),
    "与田剛": ("Yoda", "Tsuyoshi"),
    "今中慎二": ("Imanaka", "Shinji"),
    "川上憲伸": ("Kawakami", "Kenshin"),
    "吉見一起": ("Yoshimi", "Kazuki"),
    "権藤博": ("Gondo", "Hiroshi"),
    "一枝修平": ("Hitoeda", "Shuhei"),
    "山田久志": ("Yamada", "Hisashi"),
    "真弓明信": ("Mayumi", "Akinobu"),
    "梨田昌孝": ("Nashida", "Masataka"),
    "大石大二郎": ("Oishi", "Daijiro"),
    "中西清起": ("Nakanishi", "Kiyooki"),
    "桧山進次郎": ("Hiyama", "Shinjiro"),
    "浜名千広": ("Hamana", "Chihiro"),
    "今岡真訪": ("Imaoka", "Masami"),
    "鳥谷敬": ("Toritani", "Takashi"),
    "平石洋介": ("Hiraishi", "Yosuke"),
    "上原浩治": ("Uehara", "Koji"),
    "谷繁元信": ("Tanishige", "Motonobu"),
    "宮本慎也": ("Miyamoto", "Shinya"),
    # Same reading as Ogata Koichi of Hiroshima (緒方孝市), so add the team to tell them apart.
    "緒方耕一": "Ogata Koichi G",
    "佐々木主浩": ("Sasaki", "Kazuhiro"),
    "渡辺久信": ("Watanabe", "Hisanobu"),
    "田村藤夫": ("Tamura", "Fujio"),
    "篠塚和典": ("Shinozuka", "Kazunori"),
    "西本聖": ("Nishimoto", "Takashi"),
    "岩田稔": ("Iwata", "Minoru"),
    "山本昌": ("Yamamoto", "Masa"),
    "福原忍": ("Fukuhara", "Shinobu"),
    "藤田平": ("Fujita", "Taira"),
    "西山秀二": ("Nishiyama", "Shuji"),
    "井川慶": ("Igawa", "Kei"),
    "糸井嘉男": ("Itoi", "Yoshio"),
    "佐藤義則": ("Sato", "Yoshinori"),
    "安仁屋宗八": ("Aniya", "Sohachi"),
    "狩野恵輔": ("Kano", "Keisuke"),
    "谷佳知": ("Tani", "Yoshitomo"),
    "岡義朗": ("Oka", "Yoshiro"),
    "中田良弘": ("Nakata", "Yoshihiro"),
    "横山竜士": ("Yokoyama", "Ryuji"),
    "土井正博": ("Doi", "Masahiro"),
    "上田二朗": ("Ueda", "Jiro"),
    "黒田正宏": ("Kuroda", "Masahiro"),
    "田尾安志": ("Tao", "Yasushi"),
    "八木裕": ("Yagi", "Yutaka"),
    "藪恵壹": ("Yabu", "Keiichi"),
    "若松勉": ("Wakamatsu", "Tsutomu"),
    "小早川毅彦": ("Kobayakawa", "Takehiko"),
    "高津臣吾": ("Takatsu", "Shingo"),
    "小川淳司": ("Ogawa", "Atsushi"),
    "真中満": ("Manaka", "Mitsuru"),
    "荒木大輔": ("Araki", "Daisuke"),
    "井口資仁": ("Iguchi", "Tadahito"),
    "濱中治": ("Hamanaka", "Osamu"),
    "髙橋尚成": ("Takahashi", "Hisanori"),
    "高橋由伸": ("Takahashi", "Yoshinobu"),
    "前田智徳": ("Maeda", "Tomonori"),
    "村田真一": ("Murata", "Shinichi"),
    "堀内恒夫": ("Horiuchi", "Tsuneo"),
    "掛布雅之": ("Kakefu", "Masayuki"),
    "宮本和知": ("Miyamoto", "Kazutomo"),
    "清水隆行": ("Shimizu", "Takayuki"),
    "福本豊": ("Fukumoto", "Yutaka"),
    "金村義明": ("Kanemura", "Yoshiaki"),
    "岩本勉": ("Iwamoto", "Tsutomu"),
    "谷沢健一": ("Yazawa", "Kenichi"),
    "大矢明彦": ("Oya", "Akihiko"),
    "斎藤雅樹": ("Saito", "Masaki"),
    "佐伯貴弘": ("Saeki", "Takahiro"),
    "今成亮太": ("Imanari", "Ryota"),
    "柴田勲": ("Shibata", "Isao"),
    "五十嵐亮太": ("Igarashi", "Ryota"),
}

NOT_PUNDITS = {"(昨年順位)", "(OP戦順位)", "解説者"}


def norm(text):
    return unicodedata.normalize("NFKC", text)


def username_for(name_ja):
    entry = ROMAJI[norm(name_ja)] if norm(name_ja) in ROMAJI else ROMAJI[name_ja]
    if isinstance(entry, str):
        return entry
    family, given = entry
    full = f"{family} {given}"
    return full if len(full.encode()) <= MAX_USERNAME_BYTES else f"{family} {given[0]}."


def cells(line):
    return [c.strip() for c in line.strip().strip("|").split("|")]


def central_league_rows(md_text):
    lines = md_text.split("\n")
    start = next(i for i, l in enumerate(lines) if l.startswith("| 解説者") and "1位" in l)
    rows = []
    for line in lines[start:]:
        if not line.startswith("|"):
            break
        rows.append(cells(line))
    return rows


def main(path):
    # Keys are compared after NFKC so half-width kana on the page match the table above.
    global ROMAJI
    ROMAJI = {norm(k): v for k, v in ROMAJI.items()}

    rows = central_league_rows(Path(path).read_text(encoding="utf-8"))
    people = {}  # base name -> (name_ja, order)
    skipped = []
    for row in rows[2:]:  # skip header and separator
        # Strip the circled version numbers first: normalizing turns them into plain digits.
        name = norm(re.sub(r"[①②③④⑤]", "", row[0]))
        if name in {norm(x) for x in NOT_PUNDITS}:
            continue
        teams = [TEAM_INDEX.get(norm(re.sub(r"[*\s]", "", c))) for c in row[1:7]]
        if None in teams or sorted(teams) != list(range(6)):
            skipped.append(name)
            continue
        people.setdefault(name, teams)  # first row wins

    missing = [n for n in people if n not in ROMAJI]
    if missing:
        sys.exit(f"no romaji for: {missing}")

    out, seen = [], {}
    for name, teams in people.items():
        user = username_for(name)
        assert user.isascii(), user
        assert len(user.encode()) <= MAX_USERNAME_BYTES, f"{user!r} is too long"
        if user in seen:
            sys.exit(f"duplicate username {user!r}: {seen[user]} and {name}")
        seen[user] = name
        out.append([user, name] + teams)

    dest = Path("data/2026_cl_predictions.csv")
    dest.parent.mkdir(exist_ok=True)
    with dest.open("w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(["username", "name_ja", "rank1", "rank2", "rank3", "rank4", "rank5", "rank6"])
        w.writerows(out)

    abbreviated = [(u, n) for u, n in seen.items() if u.endswith(".") and u != "G.G. Sato"]
    print(f"wrote {len(out)} predictions to {dest}")
    print(f"skipped incomplete rows: {skipped}")
    print(f"first names shortened to an initial ({len(abbreviated)}):")
    for u, n in abbreviated:
        print(f"  {u:<16} {n}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    main(sys.argv[1])
