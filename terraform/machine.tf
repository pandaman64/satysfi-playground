resource "aws_security_group" "ssh_and_egress" {
  ingress {
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  ingress {
    from_port   = 8080
    to_port     = 8080
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_key_pair" "generated_key" {
  key_name   = "pan@nasu"
  public_key = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJyczFsal0IR2WI8Rghp6XXn3ZdmWzvN50UV4XPqzHhE pan@nasu"
}

data "aws_iam_policy_document" "ec2_assume_role_policy" {
  statement {
    actions = [
      "sts:AssumeRole"
    ]
    effect = "Allow"
    principals {
      type        = "Service"
      identifiers = ["ec2.amazonaws.com"]
    }
  }
}

resource "aws_iam_role" "ec2_role" {
  name               = "satysfi_playground_ec2_role"
  assume_role_policy = data.aws_iam_policy_document.ec2_assume_role_policy.json
  # Remove all inline policies
  inline_policy {}
}

resource "aws_iam_instance_profile" "machine_profile" {
  name = "satysfi_playground_ec2_instance_profile"
  role = aws_iam_role.ec2_role.name
}

# This policy is attatched to the IAM role above
data "aws_iam_policy_document" "ec2_allow_s3_access_document" {
  statement {
    actions = [
      "s3:GetObject",
      "s3:PutObject"
    ]
    effect = "Allow"
    resources = [
      "arn:aws:s3:::satysfi-playground",
      "arn:aws:s3:::satysfi-playground/*"
    ]
  }
}

resource "aws_iam_policy" "ec2_allow_s3_access_policy" {
  name   = "satysfi_playground_ec2_access_s3"
  policy = data.aws_iam_policy_document.ec2_allow_s3_access_document.json
}

resource "aws_iam_role_policy_attachment" "ec2_allow_s3_access_attach" {
  role       = aws_iam_role.ec2_role.name
  policy_arn = aws_iam_policy.ec2_allow_s3_access_policy.arn
}

resource "aws_instance" "machine" {
  # https://github.com/NixOS/nixpkgs/blob/c52ea537b37afe1e2a4fcd33f4a8a5259a2da0ce/nixos/modules/virtualisation/amazon-ec2-amis.nix#L418
  # "21.11".ap-northeast-1.x86_64-linux.hvm-ebs = "ami-07c95eda953bf5435";
  ami                  = "ami-07c95eda953bf5435"
  instance_type        = "t1.micro"
  security_groups      = [aws_security_group.ssh_and_egress.name]
  key_name             = aws_key_pair.generated_key.key_name
  iam_instance_profile = aws_iam_instance_profile.machine_profile.name

  root_block_device {
    volume_size = 10 # GiB
  }
}
