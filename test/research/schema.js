import * as z from "zod";
import fs from "fs";

const raw = fs.readFileSync("./login/09-map.json", "utf8");
const example = JSON.parse(raw);

const mapSchema = z.object({
  msg: z.literal("map"),
  clear: z.boolean().meta({ description: "unknown" }),
  player_on_level: z.boolean().meta({ description: "unknown" }),
  vgrdc: z
    .object({
      x: z.int(),
      y: z.int(),
    })
    .meta({ description: "unknown" }),
  cells: z.array(
    z.object({
      x: z.int().optional().meta({
        description:
          "relative x position of the described cell, when omitted x = last cell x-value + cells without x",
      }),
      y: z.int().optional().meta({
        description:
          "relative x position of the described cell, when omitted y = last cell y-value",
      }),
      f: z.int().optional().meta({ description: "cell feature id" }),
      mf: z.int().optional().meta({
        description: "minimap feature color, see minimap_colours in minimap.js",
      }),
      g: z.string().length(1).optional().meta({
        description: "glyph used to describe a dungeon feature, see glyphs.md",
      }),
      col: z
        .int()
        .min(0)
        .max(255)
        .optional()
        .meta({ description: "vga 8-bit color of this cell" }),
      t: z
        .object({
          bg: z.int().meta({
            description:
              "glyph variant identifier used to display different sprites for the same glyph in tiles and webtiles",
          }),
        })
        .optional(),
    }),
  ),
});

const schema = z.object({
  msgs: z.array(z.xor([mapSchema])),
});

schema.parse(example);
