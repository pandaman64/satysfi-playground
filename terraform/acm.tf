# SSL Certificates
resource "aws_acm_certificate" "api_satysfi-playground_tech" {
  domain_name               = local.api_server_domain_name
  subject_alternative_names = []
  validation_method         = "DNS"

  lifecycle {
    create_before_destroy = true
  }
}

resource "aws_route53_record" "domain_validation_api_satysfi-playground_tech" {
  for_each = {
    for dvo in aws_acm_certificate.api_satysfi-playground_tech.domain_validation_options : dvo.domain_name => {
      name   = dvo.resource_record_name
      type   = dvo.resource_record_type
      record = dvo.resource_record_value
    }
  }

  allow_overwrite = true
  name            = each.value.name
  records         = [each.value.record]
  ttl             = 60
  type            = each.value.type
  zone_id         = aws_route53_zone.satysfi-playground_tech.zone_id
}

resource "aws_acm_certificate_validation" "cert_validation_api_satysfi-playground_tech" {
  certificate_arn         = aws_acm_certificate.api_satysfi-playground_tech.arn
  validation_record_fqdns = [for record in aws_route53_record.domain_validation_api_satysfi-playground_tech : record.fqdn]
}