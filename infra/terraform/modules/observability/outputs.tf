output "sns_topic_arn" {
  description = "ARN of the alerts SNS topic. Subscribe PagerDuty/Slack/email here."
  value       = aws_sns_topic.alerts.arn
}

output "alarm_names" {
  description = "All CloudWatch alarm names created."
  value = [
    aws_cloudwatch_metric_alarm.service_down.alarm_name,
    aws_cloudwatch_metric_alarm.unhealthy_hosts.alarm_name,
    aws_cloudwatch_metric_alarm.target_5xx.alarm_name,
    aws_cloudwatch_metric_alarm.elb_5xx.alarm_name,
    aws_cloudwatch_metric_alarm.cpu_high.alarm_name,
    aws_cloudwatch_metric_alarm.memory_high.alarm_name,
  ]
}
