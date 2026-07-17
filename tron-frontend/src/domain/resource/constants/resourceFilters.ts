import type {
  ResourceContextFilter,
  ResourceLifecycleFilter,
  ResourceValidationFilter,
} from "@/domain/resource/model";

export const resourceLifecycleFilters: ResourceLifecycleFilter[] = ["ACTIVE", "DRAFT", "HISTORICAL", "ALL"];

export const resourceSourceFilters: ResourceContextFilter[] = ["ALL", "PLATFORM", "LOCAL", "INHERITED", "OVERRIDE"];

export const resourceScopeFilters: ResourceContextFilter[] = ["ALL", "PLATFORM", "WORKSPACE"];

export const resourceValidationFilters: ResourceValidationFilter[] = ["ALL", "VALID", "INVALID"];
