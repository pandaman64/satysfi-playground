resource "aws_route53_zone" "satysfi-playground_tech" {
  name = local.domain_name
}

resource "aws_route53_record" "satysfi-playground_tech" {
  zone_id = aws_route53_zone.satysfi-playground_tech.zone_id
  name    = local.api_server_domain_name
  type    = "A"

  alias {
    name                   = aws_lb.api_satysfi-playground_tech.dns_name
    zone_id                = aws_lb.api_satysfi-playground_tech.zone_id
    evaluate_target_health = true
  }
}
