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

resource "aws_security_group" "ssh_and_egress" {
  ingress {
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  ingress {
    from_port   = 80
    to_port     = 80
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

resource "aws_instance" "machine" {
  # https://github.com/NixOS/nixpkgs/blob/c52ea537b37afe1e2a4fcd33f4a8a5259a2da0ce/nixos/modules/virtualisation/amazon-ec2-amis.nix#L418
  # "21.11".ap-northeast-1.x86_64-linux.hvm-ebs = "ami-07c95eda953bf5435";
  ami             = "ami-07c95eda953bf5435"
  instance_type   = "t1.micro"
  security_groups = [aws_security_group.ssh_and_egress.name]
  key_name        = aws_key_pair.generated_key.key_name

  root_block_device {
    volume_size = 10 # GiB
  }
}

output "public_ip" {
  value = aws_instance.machine.public_ip
}