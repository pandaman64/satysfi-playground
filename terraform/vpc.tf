resource "aws_vpc" "main" {
  cidr_block = "10.0.0.0/16"
}

resource "aws_internet_gateway" "public" {
  vpc_id = aws_vpc.main.id
}

resource "aws_subnet" "public" {
  for_each = {
    for subnet in local.public_subnets : subnet.availability_zone => {
      availability_zone = subnet.availability_zone
      cidr_block        = subnet.cidr_block
    }
  }

  vpc_id            = aws_vpc.main.id
  availability_zone = each.value.availability_zone
  cidr_block        = each.value.cidr_block
}

resource "aws_subnet" "machine" {
  vpc_id            = aws_vpc.main.id
  availability_zone = "ap-northeast-1a"
  cidr_block        = "10.0.10.0/24"
}

resource "aws_route_table" "main" {
  vpc_id = aws_vpc.main.id

  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.public.id
  }

  route {
    ipv6_cidr_block = "::/0"
    gateway_id      = aws_internet_gateway.public.id
  }
}

resource "aws_route_table_association" "public" {
  for_each = aws_subnet.public

  subnet_id      = each.value.id
  route_table_id = aws_route_table.main.id
}

resource "aws_route_table_association" "machine" {
  subnet_id      = aws_subnet.public[local.machine_availability_zone].id
  route_table_id = aws_route_table.main.id
}