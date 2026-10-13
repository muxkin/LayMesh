// Same-family aliases only. UI and regional families deliberately stay separate.
// Sources: Microsoft Typography font list, Apple macOS font list, Adobe Source Han.
export const canonicalFont = (s) =>
  String(s || "")
    .normalize("NFKC")
    .toLowerCase()
    .replace(/[\s_-]/g, "");
const families = [
  ["SimSun", "宋体"],
  ["NSimSun", "新宋体"],
  ["SimHei", "黑体"],
  ["KaiTi", "楷体"],
  ["FangSong", "仿宋"],
  ["DengXian", "等线"],
  ["DengXian Light", "等线 Light"],
  ["Microsoft YaHei", "微软雅黑"],
  ["Microsoft YaHei UI", "微软雅黑 UI"],
  ["Microsoft JhengHei", "微軟正黑體", "微软正黑体"],
  ["Microsoft JhengHei UI", "微軟正黑體 UI", "微软正黑体 UI"],
  ["MingLiU", "細明體", "细明体"],
  ["PMingLiU", "新細明體", "新细明体"],
  ["DFKai-SB", "標楷體", "标楷体"],
  ["YouYuan", "幼圆"],
  ["LiSu", "隶书"],
  ["STSong", "华文宋体"],
  ["STFangsong", "华文仿宋"],
  ["STKaiti", "华文楷体"],
  ["STHeiti", "华文黑体"],
  ["STXihei", "华文细黑"],
  ["STZhongsong", "华文中宋"],
  ["STXinwei", "华文新魏"],
  ["STXingkai", "华文行楷"],
  ["STLiti", "华文隶书"],
  ["STHupo", "华文琥珀"],
  ["STCaiyun", "华文彩云"],
  ["PingFang SC", "苹方-简"],
  ["PingFang TC", "蘋方-繁"],
  ["PingFang HK", "蘋方-港"],
  ["Songti SC", "宋体-简"],
  ["Songti TC", "宋體-繁"],
  ["Kaiti SC", "楷体-简"],
  ["Kaiti TC", "楷體-繁"],
  ["Heiti SC", "黑体-简"],
  ["Heiti TC", "黑體-繁"],
  ["Source Han Sans SC", "思源黑体"],
  ["Source Han Sans TC", "思源黑體"],
  ["Source Han Sans HK", "思源黑體 香港"],
  ["Source Han Serif SC", "思源宋体"],
  ["Source Han Serif TC", "思源宋體"],
  ["Source Han Serif HK", "思源宋體 香港"],
];
const aliases = new Map(
  families.flatMap((group) =>
    group.map((name) => [canonicalFont(name), group]),
  ),
);
export const fontAliases = (name) => aliases.get(canonicalFont(name)) || [name];
export const fontFamilyKey = (name) => canonicalFont(fontAliases(name)[0]);
export const chineseFontName = (name) => fontAliases(name)[1] || name;
export function fontWeight(f) {
  if (f.weight) return f.weight;
  const s = String(f.style || "").toLowerCase();
  return /thin|细体/.test(s)
    ? 100
    : /extra.?light|ultra.?light/.test(s)
      ? 200
      : /light/.test(s)
        ? 300
        : /semi.?bold|demi/.test(s)
          ? 600
          : /extra.?bold|ultra.?bold/.test(s)
            ? 800
            : /black|heavy/.test(s)
              ? 900
              : /bold|粗/.test(s)
                ? 700
                : /medium/.test(s)
                  ? 500
                  : 400;
}
export const fontItalic = (f) =>
  f.italic ?? /italic|oblique|斜/i.test(f.style || "");
