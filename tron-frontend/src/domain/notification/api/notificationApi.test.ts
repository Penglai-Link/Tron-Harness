import { expect, it } from "vitest";
import { notificationTarget, type PlatformNotification } from "./notificationApi";
const notification: PlatformNotification = { id: "n", dudeCode: "main-dude", title: "Done", body: "", category: "completion", target: {}, readAt: null, resolvedAt: null, archivedAt: null, createdAt: 1 };
it("routes candidate approvals to the exact version review screen", () => { expect(notificationTarget({ ...notification, target: { candidateCode: "skill/one" } })).toBe("/settings/skills?tab=approvals&candidateCode=skill%2Fone"); });
it("routes a work session under its owning Dude", () => { expect(notificationTarget({ ...notification, target: { sessionCode: "background" } })).toBe("/dudes?dude=main-dude&session=background&tab=history"); });
it("rejects external notification destinations", () => { expect(notificationTarget({ ...notification, dudeCode: null, target: { href: "//external.test" } })).toBeNull(); expect(notificationTarget({ ...notification, dudeCode: null, target: { href: "https://external.test" } })).toBeNull(); });

it("prioritizes the actionable Thread over its origin session and task", () => { expect(notificationTarget({ ...notification, target: { originSessionCode: "origin", sessionCode: "thread-a", sessionKind: "Thread", taskCode: "task-a" } })).toBe("/dudes?dude=main-dude&session=thread-a&tab=work"); });

it("opens completed work history when a result includes its task and session", () => { expect(notificationTarget({ ...notification, target: { taskCode: "task-a", sessionCode: "work-session" } })).toBe("/dudes?dude=main-dude&session=work-session&tab=history"); });
it("opens the task when the notification has no execution session", () => { expect(notificationTarget({ ...notification, target: { taskCode: "task-a" } })).toBe("/dudes?dude=main-dude&task=task-a&tab=tasks"); });
it("opens the Skill candidate ahead of all execution targets", () => { expect(notificationTarget({ ...notification, target: { candidateCode: "skill-a", sessionCode: "thread-a", originSessionCode: "work-session", sessionKind: "Thread", taskCode: "task-a" } })).toBe("/settings/skills?tab=approvals&candidateCode=skill-a"); });
