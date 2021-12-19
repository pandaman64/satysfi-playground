data "aws_iam_policy_document" "share_allow_only_cloudfront" {
  statement {
    actions = [
      "s3:GetObject"
    ]
    effect = "Allow"
    resources = [
      "${aws_s3_bucket.share.arn}",
      "${aws_s3_bucket.share.arn}/*"
    ]
    principals {
      type        = "AWS"
      identifiers = [aws_cloudfront_origin_access_identity.share.iam_arn]
    }
  }
}

resource "aws_s3_bucket" "share" {
  bucket = "satysfi-playground"
  acl    = "private"
}

resource "aws_s3_bucket_policy" "share" {
  bucket = aws_s3_bucket.share.id
  policy = data.aws_iam_policy_document.share_allow_only_cloudfront.json
}