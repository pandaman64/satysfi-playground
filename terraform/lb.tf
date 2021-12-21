resource "aws_security_group" "public" {
  vpc_id = aws_vpc.main.id

  ingress {
    from_port   = 443
    to_port     = 443
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
    cidr_blocks = [aws_subnet.public[local.machine_availability_zone].cidr_block]
  }
}

resource "aws_lb" "api_satysfi-playground_tech" {
  internal           = false
  load_balancer_type = "application"
  security_groups    = [aws_security_group.public.id]
  subnets            = [for subnet in aws_subnet.public : subnet.id]

  # TODO: access_logs
}

resource "aws_lb_target_group" "api_satysfi-playground_tech" {
  port        = 8080
  protocol    = "HTTP"
  target_type = "instance"
  vpc_id      = aws_vpc.main.id

  health_check {
    path     = "/healthcheck"
    matcher  = "200"
    port     = 8080
    protocol = "HTTP"
  }
}

resource "aws_lb_target_group_attachment" "api_satysfi-playground_tech" {
  target_group_arn = aws_lb_target_group.api_satysfi-playground_tech.arn
  target_id        = aws_instance.machine.id
  port             = 8080
}

resource "aws_lb_listener" "api_satysfi-playground_tech" {
  load_balancer_arn = aws_lb.api_satysfi-playground_tech.arn
  port              = 443
  protocol          = "HTTPS"
  certificate_arn   = aws_acm_certificate.api_satysfi-playground_tech.arn

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.api_satysfi-playground_tech.arn
  }
}

resource "aws_lb_listener" "http_to_https" {
  load_balancer_arn = aws_lb.api_satysfi-playground_tech.arn
  port              = 80
  protocol          = "HTTP"

  default_action {
    type = "redirect"

    redirect {
      port        = 443
      protocol    = "HTTPS"
      status_code = "HTTP_301"
    }
  }
}