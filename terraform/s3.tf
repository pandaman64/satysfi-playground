data "aws_iam_policy_document" "share" {
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

resource "aws_s3_bucket" "share" {
  bucket = "satysfi-playground"
  acl    = "private"
  policy = data.aws_iam_policy_document.share.json
}