import { expect, test } from "@playwright/test";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const thisFilePath = fileURLToPath(import.meta.url);

function source(pathFromFrontendRoot: string): string {
  return readFileSync(
    resolve(thisFilePath, "..", "..", pathFromFrontendRoot),
    "utf8",
  );
}

test("TDD-UM-08: admin user model uses stable user-management boundary fields", async () => {
  const modelSource = source("src/domain/admin/model/AdminModel.ts");

  expect(modelSource).toContain("userCode");
  expect(modelSource).toContain("username");
  expect(modelSource).toContain("roleCodes");
  expect(modelSource).toContain("spaceCodes");

  expect(modelSource).not.toContain("workspaces:");
  expect(modelSource).not.toContain("roles:");
});

test("TDD-UM-08: admin user statuses are narrowed to Active or Archived only", async () => {
  const mockDataSource = source("src/domain/admin/mock/data.ts");
  const usersArrayBlock = mockDataSource.match(
    /const users:\s*UserRecord\[\]\s*=\s*\[(?<users>[\s\S]*?)\];/,
  )?.groups?.users;
  expect(usersArrayBlock).toBeDefined();

  const statusLiterals = Array.from(
    usersArrayBlock!.matchAll(/status:\s*"([^"]+)"/g),
    (match) => match[1],
  );

  expect(statusLiterals.length).toBeGreaterThan(0);
  for (const status of statusLiterals) {
    expect(["Active", "Archived"]).toContain(status);
  }
});

test("TDD-UM-08: user detail navigation is keyed by userCode query boundary", async () => {
  const usersPageSource = source("src/domain/admin/pages/UsersPage.tsx");
  const detailsPageSource = source("src/domain/admin/pages/UserDetailsPage.tsx");

  expect(usersPageSource).toContain("/admin/users/detail?userCode=");
  expect(detailsPageSource).toContain("get(\"userCode\")");
});
