import { expect, it } from "vitest";
import { notificationTarget, type PlatformNotification } from "./notificationApi";
const notification: PlatformNotification = { id: "n", dudeCode: "main-dude", title: "Done", body: "", category: "completion", target: {}, readAt: null, resolvedAt: null, archivedAt: null, createdAt: 1 };
it("routes candidate approvals to the exact version review screen", () => { expect(notificationTarget({ ...notification, target: { candidateCode: "skill/one" } })).toBe("/settings/skills?tab=approvals&candidateCode=skill%2Fone"); });
it("routes a work session under its owning Dude", () => { expect(notificationTarget({ ...notification, target: { sessionCode: "background" } })).toBe("/dudes?dude=main-dude&session=background&tab=history"); });
it("rejects external notification destinations", () => { expect(notificationTarget({ ...notification, dudeCode: null, target: { href: "//external.test" } })).toBeNull(); expect(notificationTarget({ ...notification, dudeCode: null, target: { href: "https://external.test" } })).toBeNull(); });

it("prioritizes the actionable Thread over its origin session and task", () => { expect(notificationTarget({ ...notification, target: { sessionCode: "origin", threadCode: "thread-a", taskCode: "task-a" } })).toBe("/dudes?dude=main-dude&thread=thread-a&tab=work"); });
