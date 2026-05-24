import * as cdk from "aws-cdk-lib";
import { Construct } from "constructs";
import * as path from "path";
import * as lambda from "aws-cdk-lib/aws-lambda";
import * as events from "aws-cdk-lib/aws-events";
import * as targets from "aws-cdk-lib/aws-events-targets";
import * as iam from "aws-cdk-lib/aws-iam";
import * as logs from "aws-cdk-lib/aws-logs";
import { RustFunction } from "cargo-lambda-cdk";

export interface MonitoringStackProps extends cdk.StackProps {
  targetUrl: string;
  emailFrom: string;
  emailTo: string;
  userAgent: string;
  webhookParamName: string;
  availabilityThreshold: string;
}

export class MonitoringStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props: MonitoringStackProps) {
    super(scope, id, props);

    const fn = new RustFunction(this, "MonitorFn", {
      manifestPath: path.join(__dirname, "..", "..", "lambda", "Cargo.toml"),
      architecture: lambda.Architecture.ARM_64,
      memorySize: 256,
      timeout: cdk.Duration.seconds(30),
      logRetention: logs.RetentionDays.ONE_WEEK,
      environment: {
        TARGET_URL: props.targetUrl,
        EMAIL_FROM: props.emailFrom,
        EMAIL_TO: props.emailTo,
        USER_AGENT: props.userAgent,
        WEBHOOK_PARAM_NAME: props.webhookParamName,
        AVAILABILITY_THRESHOLD: props.availabilityThreshold,
        RUST_LOG: "info",
      },
    });

    fn.addToRolePolicy(
      new iam.PolicyStatement({
        actions: ["ses:SendEmail"],
        resources: [`arn:aws:ses:${this.region}:${this.account}:identity/${props.emailFrom}`],
      }),
    );

    fn.addToRolePolicy(
      new iam.PolicyStatement({
        actions: ["ssm:GetParameter"],
        resources: [
          `arn:aws:ssm:${this.region}:${this.account}:parameter${props.webhookParamName}`,
        ],
      }),
    );

    fn.addToRolePolicy(
      new iam.PolicyStatement({
        actions: ["kms:Decrypt"],
        resources: ["*"],
        conditions: {
          StringEquals: {
            "kms:ViaService": `ssm.${this.region}.amazonaws.com`,
          },
        },
      }),
    );

    new events.Rule(this, "MonitorSchedule", {
      schedule: events.Schedule.rate(cdk.Duration.minutes(5)),
      targets: [new targets.LambdaFunction(fn, { retryAttempts: 0 })],
    });
  }
}
