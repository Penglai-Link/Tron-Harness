-- Remove the unfinished built-in page from both current and legacy menu trees.
DELETE FROM navigation_menu_tree_items
WHERE menu_id IN (SELECT id FROM navigation_menus WHERE menu_code = 'OBSERVABILITY' AND source = 'BuiltIn');
DELETE FROM navigation_domain_menu_items
WHERE menu_id IN (SELECT id FROM navigation_menus WHERE menu_code = 'OBSERVABILITY' AND source = 'BuiltIn');
DELETE FROM navigation_menu_translations
WHERE menu_id IN (SELECT id FROM navigation_menus WHERE menu_code = 'OBSERVABILITY' AND source = 'BuiltIn');
DELETE FROM navigation_menus WHERE menu_code = 'OBSERVABILITY' AND source = 'BuiltIn';
