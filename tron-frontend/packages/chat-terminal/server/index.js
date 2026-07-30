import "reflect-metadata";

import { pathToFileURL } from "node:url";
import { Module } from "@nestjs/common";
import { NestFactory } from "@nestjs/core";

import { TerminalGateway } from "./scoket.js";
import { ChatController } from "./sse.js";

export class AppModule {}

Module({
  controllers: [ChatController],
  providers: [TerminalGateway],
})(AppModule);

export async function createApp() {
  const app = await NestFactory.create(AppModule, { bodyParser: false });
  app.enableCors({
    allowedHeaders: ["Content-Type", "Authorization", "X-Message-Id"],
    methods: ["POST", "OPTIONS"],
    origin: process.env.TERMINAL_DEMO_ORIGIN ?? "*",
  });
  return app;
}

export async function bootstrap() {
  const port = readPort(process.env.PORT);
  const host = process.env.HOST ?? "0.0.0.0";
  const app = await createApp();

  await app.listen(port, host);
  console.log(`NestJS 终端 demo 已启动：http://127.0.0.1:${port}`);
}

function readPort(value) {
  const port = Number(value ?? 8787);
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    throw new Error("PORT 必须是 1 到 65535 之间的整数");
  }
  return port;
}

const entryUrl = process.argv[1] ? pathToFileURL(process.argv[1]).href : undefined;
if (entryUrl === import.meta.url) {
  bootstrap().catch((error) => {
    console.error(`NestJS 终端 demo 启动失败：${error.message}`);
    process.exitCode = 1;
  });
}
