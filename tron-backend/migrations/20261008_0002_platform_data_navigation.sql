-- Platform settings owns these entries; remove persisted workspace assignments.
SAVEPOINT platform_data_navigation;
-- Detach children first because parent links use ON DELETE RESTRICT. Custom
-- child entries survive at the root, with unique temporary positions.
UPDATE navigation_menu_tree_items SET parent_item_id=NULL, position=id
WHERE parent_item_id IN (SELECT i.id FROM navigation_menu_tree_items i JOIN navigation_menus m ON m.id=i.menu_id WHERE m.source='BuiltIn' AND m.menu_code IN ('MEMORY','KNOWLEDGE_GRAPH','KNOWLEDGE_BASE','RAG','AGENTS','INTERNAL_AGENTS','SYSTEM_AGENTS'));
UPDATE navigation_domain_menu_items SET parent_item_id=NULL, position=id
WHERE parent_item_id IN (SELECT i.id FROM navigation_domain_menu_items i JOIN navigation_menus m ON m.id=i.menu_id WHERE m.source='BuiltIn' AND m.menu_code IN ('MEMORY','KNOWLEDGE_GRAPH','KNOWLEDGE_BASE','RAG','AGENTS','INTERNAL_AGENTS','SYSTEM_AGENTS'));
DELETE FROM navigation_menu_tree_items WHERE menu_id IN
 (SELECT id FROM navigation_menus WHERE source='BuiltIn' AND menu_code IN ('MEMORY','KNOWLEDGE_GRAPH','KNOWLEDGE_BASE','RAG','AGENTS','INTERNAL_AGENTS','SYSTEM_AGENTS'));
DELETE FROM navigation_domain_menu_items WHERE menu_id IN
 (SELECT id FROM navigation_menus WHERE source='BuiltIn' AND menu_code IN ('MEMORY','KNOWLEDGE_GRAPH','KNOWLEDGE_BASE','RAG','AGENTS','INTERNAL_AGENTS','SYSTEM_AGENTS'));
DELETE FROM navigation_menu_translations WHERE menu_id IN
 (SELECT id FROM navigation_menus WHERE source='BuiltIn' AND menu_code IN ('MEMORY','KNOWLEDGE_GRAPH','KNOWLEDGE_BASE','RAG','AGENTS','INTERNAL_AGENTS','SYSTEM_AGENTS'));
DELETE FROM navigation_menus WHERE source='BuiltIn' AND menu_code IN ('MEMORY','KNOWLEDGE_GRAPH','KNOWLEDGE_BASE','RAG','AGENTS','INTERNAL_AGENTS','SYSTEM_AGENTS');
RELEASE platform_data_navigation;
