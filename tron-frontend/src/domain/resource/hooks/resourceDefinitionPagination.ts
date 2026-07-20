import {
  pageResourceDefinitions,
  type ResourceDefinitionPageApiRequest,
  type ResourceDefinitionVersionApiRecord,
} from "@/domain/resource/api";

const resourceDefinitionPageSize = 50;

type ResourceDefinitionPageFilter = Omit<
  ResourceDefinitionPageApiRequest,
  "current" | "size"
>;

function recordKey(record: ResourceDefinitionVersionApiRecord) {
  return [
    record.scopeKind,
    record.scopeCode,
    record.definitionCode,
    record.definitionVersion,
  ].join(":");
}

/**
 * Loads every record matching a definition-page filter. The management API
 * caps a single page at 50 records, but definition selectors must not silently
 * omit valid definitions once a workspace grows beyond that limit.
 */
export async function pageAllResourceDefinitions(
  filter: ResourceDefinitionPageFilter,
): Promise<ResourceDefinitionVersionApiRecord[]> {
  const firstPage = await pageResourceDefinitions({
    ...filter,
    current: 1,
    size: resourceDefinitionPageSize,
  });
  const pageSize = Math.max(firstPage.size, 1);
  const pageCount = Math.max(1, Math.ceil(firstPage.total / pageSize));
  const remainingPages = await Promise.all(Array.from(
    { length: pageCount - 1 },
    (_, index) => pageResourceDefinitions({
      ...filter,
      current: index + 2,
      size: pageSize,
    }),
  ));

  const seen = new Set<string>();
  return [firstPage, ...remainingPages]
    .flatMap((page) => page.records)
    .filter((record) => {
      const key = recordKey(record);
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
}
