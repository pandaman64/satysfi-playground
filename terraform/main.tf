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

output "public_ip" {
  value = aws_instance.machine.public_ip
}

output "s3_url" {
  value = aws_s3_bucket.share.bucket_domain_name
}