-- Remove the redundant chat page menu, including its saved assignments.
-- The chat panel widgets remain the session navigation entry points.
SAVEPOINT remove_chats_navigation;
DELETE FROM navigation_menu_tree_items
WHERE menu_id IN (SELECT id FROM navigation_menus WHERE menu_code = 'CHATS' AND source = 'BuiltIn');
DELETE FROM navigation_domain_menu_items
WHERE menu_id IN (SELECT id FROM navigation_menus WHERE menu_code = 'CHATS' AND source = 'BuiltIn');
DELETE FROM navigation_menu_translations
WHERE menu_id IN (SELECT id FROM navigation_menus WHERE menu_code = 'CHATS' AND source = 'BuiltIn');
DELETE FROM navigation_menus WHERE menu_code = 'CHATS' AND source = 'BuiltIn';
RELEASE remove_chats_navigation;
