#!/usr/bin/env node
import * as cdk from "aws-cdk-lib";
import * as dotenv from "dotenv";
import * as path from "path";
import { MonitoringStack } from "../lib/infrastructure-stack";

dotenv.config({ path: path.join(__dirname, "..", ".env") });

const requireEnv = (name: string): string => {
  const v = process.env[name];
  if (!v) {
    throw new Error(`Required env var ${name} is missing. Set it in
  infrastructure/.env`);
  }
  return v;
};

const emailFrom = requireEnv("EMAIL_FROM");
const emailTo = requireEnv("EMAIL_TO");

const app = new cdk.App();
new MonitoringStack(app, "SozenMonitoringStack", {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: "ap-northeast-1",
  },
  targetUrl: "https://reserva.be/sugamo401",
  emailFrom,
  emailTo,
  userAgent: "sozen-monitor/0.1 (+contact: )",
  webhookParamName: "/sozen-monitor/webhooks",
});
