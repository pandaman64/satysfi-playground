resource "aws_route53_zone" "satysfi-playground_tech" {
  name = local.domain_name
}

resource "aws_route53_record" "satysfi-playground_tech" {
  zone_id = aws_route53_zone.satysfi-playground_tech.zone_id
  name    = local.api_server_domain_name
  type    = "A"
  ttl     = 60
  records = [
    aws_eip.machine.public_ip
  ]
}

resource "aws_route53_record" "vercel_apex" {
  zone_id = aws_route53_zone.satysfi-playground_tech.zone_id
  name    = local.domain_name
  records = [local.frontend_ip]
  ttl     = 60
  type    = "A"
}

resource "aws_route53_record" "vercel_www" {
  zone_id = aws_route53_zone.satysfi-playground_tech.zone_id
  name    = "www.${local.domain_name}"
  records = [local.frontend_www_cname]
  ttl     = 60
  type    = "CNAME"
}