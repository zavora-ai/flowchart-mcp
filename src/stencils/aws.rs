//! AWS (mxgraph.aws4) stencil entries. `path` is the bare resource-icon name;
//! the resolver wraps it as `resourceIcon`/`resIcon`.

use super::Entry;

pub const ENTRIES: &[Entry] = &[
    // Compute
    Entry { key: "aws.ec2", path: "mxgraph.aws4.ec2", desc: "EC2 instance" },
    Entry { key: "aws.lambda", path: "mxgraph.aws4.lambda", desc: "Lambda function" },
    Entry { key: "aws.ecs", path: "mxgraph.aws4.elastic_container_service", desc: "ECS" },
    Entry { key: "aws.eks", path: "mxgraph.aws4.elastic_kubernetes_service", desc: "EKS" },
    Entry { key: "aws.fargate", path: "mxgraph.aws4.fargate", desc: "Fargate" },
    Entry { key: "aws.batch", path: "mxgraph.aws4.batch", desc: "AWS Batch" },
    Entry { key: "aws.elastic_beanstalk", path: "mxgraph.aws4.elastic_beanstalk", desc: "Elastic Beanstalk" },
    Entry { key: "aws.lightsail", path: "mxgraph.aws4.lightsail", desc: "Lightsail" },
    Entry { key: "aws.auto_scaling", path: "mxgraph.aws4.auto_scaling", desc: "Auto Scaling" },
    // Storage
    Entry { key: "aws.s3", path: "mxgraph.aws4.s3", desc: "S3 bucket" },
    Entry { key: "aws.ebs", path: "mxgraph.aws4.elastic_block_store", desc: "EBS volume" },
    Entry { key: "aws.efs", path: "mxgraph.aws4.elastic_file_system", desc: "EFS" },
    Entry { key: "aws.glacier", path: "mxgraph.aws4.s3_glacier", desc: "S3 Glacier" },
    Entry { key: "aws.storage_gateway", path: "mxgraph.aws4.storage_gateway", desc: "Storage Gateway" },
    // Database
    Entry { key: "aws.rds", path: "mxgraph.aws4.rds", desc: "RDS database" },
    Entry { key: "aws.aurora", path: "mxgraph.aws4.aurora", desc: "Aurora" },
    Entry { key: "aws.dynamodb", path: "mxgraph.aws4.dynamodb", desc: "DynamoDB" },
    Entry { key: "aws.elasticache", path: "mxgraph.aws4.elasticache", desc: "ElastiCache" },
    Entry { key: "aws.redshift", path: "mxgraph.aws4.redshift", desc: "Redshift" },
    Entry { key: "aws.documentdb", path: "mxgraph.aws4.documentdb_with_mongodb_compatibility", desc: "DocumentDB" },
    Entry { key: "aws.neptune", path: "mxgraph.aws4.neptune", desc: "Neptune" },
    // Networking
    Entry { key: "aws.vpc", path: "mxgraph.aws4.virtual_private_cloud", desc: "VPC" },
    Entry { key: "aws.api_gateway", path: "mxgraph.aws4.api_gateway", desc: "API Gateway" },
    Entry { key: "aws.cloudfront", path: "mxgraph.aws4.cloudfront", desc: "CloudFront CDN" },
    Entry { key: "aws.route53", path: "mxgraph.aws4.route_53", desc: "Route 53" },
    Entry { key: "aws.elb", path: "mxgraph.aws4.elastic_load_balancing", desc: "Elastic Load Balancing" },
    Entry { key: "aws.direct_connect", path: "mxgraph.aws4.direct_connect", desc: "Direct Connect" },
    Entry { key: "aws.transit_gateway", path: "mxgraph.aws4.transit_gateway", desc: "Transit Gateway" },
    Entry { key: "aws.nat_gateway", path: "mxgraph.aws4.nat_gateway", desc: "NAT Gateway" },
    // Integration / messaging
    Entry { key: "aws.sns", path: "mxgraph.aws4.simple_notification_service", desc: "SNS" },
    Entry { key: "aws.sqs", path: "mxgraph.aws4.simple_queue_service", desc: "SQS" },
    Entry { key: "aws.eventbridge", path: "mxgraph.aws4.eventbridge", desc: "EventBridge" },
    Entry { key: "aws.step_functions", path: "mxgraph.aws4.step_functions", desc: "Step Functions" },
    Entry { key: "aws.kinesis", path: "mxgraph.aws4.kinesis", desc: "Kinesis" },
    // Security / identity
    Entry { key: "aws.iam", path: "mxgraph.aws4.identity_and_access_management", desc: "IAM" },
    Entry { key: "aws.cognito", path: "mxgraph.aws4.cognito", desc: "Cognito" },
    Entry { key: "aws.kms", path: "mxgraph.aws4.key_management_service", desc: "KMS" },
    Entry { key: "aws.secrets_manager", path: "mxgraph.aws4.secrets_manager", desc: "Secrets Manager" },
    Entry { key: "aws.waf", path: "mxgraph.aws4.waf", desc: "WAF" },
    Entry { key: "aws.shield", path: "mxgraph.aws4.shield", desc: "Shield" },
    // Management / analytics / ml
    Entry { key: "aws.cloudwatch", path: "mxgraph.aws4.cloudwatch", desc: "CloudWatch" },
    Entry { key: "aws.cloudformation", path: "mxgraph.aws4.cloudformation", desc: "CloudFormation" },
    Entry { key: "aws.athena", path: "mxgraph.aws4.athena", desc: "Athena" },
    Entry { key: "aws.glue", path: "mxgraph.aws4.glue", desc: "Glue" },
    Entry { key: "aws.sagemaker", path: "mxgraph.aws4.sagemaker", desc: "SageMaker" },
    // Misc
    Entry { key: "aws.user", path: "mxgraph.aws4.user", desc: "User" },
    Entry { key: "aws.users", path: "mxgraph.aws4.users", desc: "Users" },
    Entry { key: "aws.internet", path: "mxgraph.aws4.internet", desc: "Internet" },
];
