-- A standalone top-level menu. Existing customized placement survives restarts.
INSERT OR IGNORE INTO navigation_menus
(id,menu_code,source,menu_type,page_code,url,icon,enabled,created_at,updated_at)
VALUES (734510000000000030,'OBSERVABILITY','BuiltIn','Internal','observability.index',NULL,'Activity',1,'2026-10-10T00:00:00Z','2026-10-10T00:00:00Z');
INSERT OR IGNORE INTO navigation_menu_translations(id,menu_id,locale,label)
SELECT 734530000000000060,id,'zh-CN','可观测' FROM navigation_menus WHERE menu_code='OBSERVABILITY';
INSERT OR IGNORE INTO navigation_menu_translations(id,menu_id,locale,label)
SELECT 734530000000000061,id,'en-US','Observability' FROM navigation_menus WHERE menu_code='OBSERVABILITY';
INSERT OR IGNORE INTO navigation_menu_tree_items(id,tree_id,menu_id,parent_item_id,position)
SELECT 734550000000000030,t.id,m.id,NULL,
    COALESCE((SELECT MAX(position)+1 FROM navigation_menu_tree_items WHERE tree_id=t.id AND parent_item_id IS NULL),0)
FROM navigation_menu_trees t,navigation_menus m WHERE t.tree_code='ADVANCED' AND m.menu_code='OBSERVABILITY';
