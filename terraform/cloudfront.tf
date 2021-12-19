resource "aws_cloudfront_origin_access_identity" "share" {
  comment = "CloudFront to S3"
}

resource "aws_cloudfront_distribution" "share" {
  origin {
    domain_name = aws_s3_bucket.share.bucket_regional_domain_name
    origin_id   = aws_s3_bucket.share.id

    s3_origin_config {
      origin_access_identity = aws_cloudfront_origin_access_identity.share.cloudfront_access_identity_path
    }
  }

  enabled         = true
  is_ipv6_enabled = true
  comment         = "CloudFront for /share"
  # TODO: logging

  default_cache_behavior {
    allowed_methods        = ["GET", "HEAD", "OPTIONS"]
    cached_methods         = ["GET", "HEAD", "OPTIONS"]
    target_origin_id       = aws_s3_bucket.share.id
    compress               = true
    viewer_protocol_policy = "redirect-to-https"

    forwarded_values {
      query_string = false

      cookies {
        forward = "none"
      }
    }
  }

  restrictions {
    geo_restriction {
      restriction_type = "none"
    }
  }

  viewer_certificate {
    cloudfront_default_certificate = true
  }
}