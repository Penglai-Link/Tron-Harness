-- Rename the built-in Topics menu to Work Topics. 0001_init.sql still seeds the
-- original label, so this migration also covers fresh databases. User-customized
-- labels and descriptions are preserved.

UPDATE navigation_menu_translations
SET label = CASE locale WHEN 'zh-CN' THEN '工作主题' ELSE 'Work Topics' END
WHERE menu_id = (SELECT id FROM navigation_menus
                 WHERE menu_code = 'CHAT_PROJECTS' AND source = 'BuiltIn')
  AND ((locale = 'zh-CN' AND label = '主题')
    OR (locale = 'en-US' AND label = 'Topics'));

UPDATE navigation_menus
SET description_zh_cn = CASE WHEN description_zh_cn = '查看主题及其关联的会话。'
        THEN '查看工作主题及其关联的会话。' ELSE description_zh_cn END,
    description_en_us = CASE WHEN description_en_us = 'Browse topics and their associated conversations.'
        THEN 'Browse work topics and their associated conversations.' ELSE description_en_us END
WHERE menu_code = 'CHAT_PROJECTS' AND source = 'BuiltIn';