data "aws_iam_policy_document" "s3_bucket_policy" {
  statement {
    actions = [
      "s3:GetObject"
    ]
    effect = "Allow"
    resources = [
      "arn:aws:s3:::satysfi-playground",
      "arn:aws:s3:::satysfi-playground/*"
    ]
    principals {
      identifiers = ["*"]
      type        = "*"
    }
  }
}

resource "aws_s3_bucket" "s3_bucket" {
  bucket = "satysfi-playground"
  acl    = "private"
  policy = data.aws_iam_policy_document.s3_bucket_policy.json
}