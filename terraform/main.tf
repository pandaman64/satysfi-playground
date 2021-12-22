# The running AWS account needs the following roles to be attached:
# - iam:*
# TODO: figure out the minimal needed roles by using CloudTrail
terraform {
  backend "remote" {
    organization = "SATySFi-Playground"
    workspaces {
      name = "satysfi-playground-cli"
    }
  }
}

provider "aws" {
  region = "ap-northeast-1"
}

locals {
  domain_name            = "satysfi-playground.tech"
  api_server_domain_name = "api.${local.domain_name}"
  # Check these config on Vercel
  frontend_ip        = "76.76.21.21"
  frontend_www_cname = "cname.vercel-dns.com"
  public_subnets = toset([
    {
      availability_zone = "ap-northeast-1a"
      cidr_block        = "10.0.1.0/24"
    },
    {
      availability_zone = "ap-northeast-1c"
      cidr_block        = "10.0.2.0/24"
    }
  ])
  machine_availability_zone = "ap-northeast-1a"
}

output "public_ip" {
  value = aws_eip.machine.public_ip
}

output "s3_region" {
  value = aws_s3_bucket.share.region
}

output "s3_public_domain_name" {
  value = aws_cloudfront_distribution.share.domain_name
}

# We need to set these name servers in Google Domains
output "route53_name_servers" {
  value = aws_route53_zone.satysfi-playground_tech.name_servers
}

output "api_domain_name" {
  value = aws_route53_record.satysfi-playground_tech.fqdn
}