-- Tag and category value colors: a named palette color, or a custom "#rrggbb".
ALTER TABLE tags DROP CONSTRAINT tags_color_check;
ALTER TABLE tags ADD CONSTRAINT tags_color_check
    CHECK (color ~ '^(gray|blue|green|amber|red|purple|teal|pink|orange|indigo|lime|cyan|fuchsia|yellow|sky|brown|#[0-9a-f]{6})$');

ALTER TABLE category_values DROP CONSTRAINT category_values_color_check;
ALTER TABLE category_values ADD CONSTRAINT category_values_color_check
    CHECK (color ~ '^(gray|blue|green|amber|red|purple|teal|pink|orange|indigo|lime|cyan|fuchsia|yellow|sky|brown|#[0-9a-f]{6})$');
