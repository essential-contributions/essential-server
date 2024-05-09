terraform {
  required_providers {
    aws = {
      source = "hashicorp/aws"
    }
  }
  required_version = ">= 1.2.0"
}

provider "aws" {
  region = "ap-southeast-2"
}

variable "rqlite_img_path" {
  description = "Path to the rqlite server image."
  type        = string
}

variable "essential_img_path" {
  description = "Path to the essential server image."
  type        = string
}

variable "etcd_img_path" {
  description = "Path to the essential server image."
  type        = string
}

# TODO, chane this to a public load balancer ip
output "public_ip" {
  value = aws_instance.essential_server.public_ip
}

output "etcd_ip" {
  value = aws_instance.etcd_server.private_ip
}

output "rqlite_ip" {
  value = aws_instance.rqlite_server.private_ip
}

# Instances 

resource "aws_instance" "rqlite_server" {
  count                  = 3
  ami                    = aws_ami.rqlite_ami.id
  instance_type          = "t2.micro"
  subnet_id              = aws_subnet.rqlite_subnet.id
  vpc_security_group_ids = [aws_security_group.sg_rqlite_subnet.id]

  tags = {
    Name = "Instance-${count.index + 1}"  # Unique name for each instance
  }
}

resource "aws_instance" "etcd_server" {
  ami                    = aws_ami.etcd_ami.id
  instance_type          = "t2.micro"
  subnet_id              = aws_subnet.rqlite_subnet.id
  vpc_security_group_ids = [aws_security_group.sg_rqlite_subnet.id]
  private_ips     = ["10.0.2.50"]
}

# resource "aws_instance" "essential_server" {
#   ami                    = aws_ami.essential_ami.id
#   instance_type          = "t2.micro"
#   subnet_id              = aws_subnet.essential_subnet.id
#   vpc_security_group_ids = [aws_security_group.sg_essential_subnet.id]
# }
resource "aws_launch_template" "essential_server" {
  name          = "essential-launch-template"
  image_id      = aws_ami.essential_ami.id  # Specify your AMI ID
  instance_type = "t2.micro"    # Specify your instance type

  block_device_mappings {
    device_name = "/dev/sda1"
    ebs {
      volume_size = 8
    }
  }
}

# Autoscaling Essential

resource "aws_autoscaling_group" "essential_server_asg" {
  launch_template {
    id      = aws_launch_template.essential_server.id
    version = "$Latest"
  }

  min_size             = 1
  max_size             = 10
  desired_capacity     = 2
  vpc_zone_identifier  = [aws_subnet.essential_subnet]

  tag {
    key                 = "Name"
    value               = "essential-server-instance"
    propagate_at_launch = true
  }
}

resource "aws_autoscaling_policy" "scale_out" {
  name                   = "scale-out"
  scaling_adjustment     = 1
  adjustment_type        = "ChangeInCapacity"
  cooldown               = 300
  autoscaling_group_name = aws_autoscaling_group.essential_server_asg.name

  alarm {
    alarm_name          = "high-cpu-usage"
    comparison_operator = "GreaterThanThreshold"
    evaluation_periods  = 2
    metric_name         = "CPUUtilization"
    namespace           = "AWS/EC2"
    period              = 120
    statistic           = "Average"
    threshold           = 75
    actions_enabled     = true
    alarm_actions       = [aws_autoscaling_policy.scale_out.arn]
  }
}

resource "aws_autoscaling_policy" "scale_in" {
  name                   = "scale-in"
  scaling_adjustment     = -1
  adjustment_type        = "ChangeInCapacity"
  cooldown               = 300
  autoscaling_group_name = aws_autoscaling_group.essential_server_asg.name

  alarm {
    alarm_name          = "low-cpu-usage"
    comparison_operator = "LessThanThreshold"
    evaluation_periods  = 2
    metric_name         = "CPUUtilization"
    namespace           = "AWS/EC2"
    period              = 120
    statistic           = "Average"
    threshold           = 25
    actions_enabled     = true
    alarm_actions       = [aws_autoscaling_policy.scale_in.arn]
  }
}


# Networking

resource "aws_vpc" "essential_vpc" {
  cidr_block = "10.0.0.0/16"
  enable_dns_support = true
  enable_dns_hostnames = true

  tags = {
    Name = "essential_vpc"
  }
}

resource "aws_subnet" "essential_subnet" {
  vpc_id     = aws_vpc.essential_vpc.id
  cidr_block = "10.0.1.0/24"
  map_public_ip_on_launch = true  # Enable public IP

  tags = {
    Name = "EssentialSubnet"
  }
}

resource "aws_subnet" "rqlite_subnet" {
  vpc_id     = aws_vpc.essential_vpc.id
  cidr_block = "10.0.2.0/24"
  map_public_ip_on_launch = false  # No public IP

  tags = {
    Name = "RqliteSubnet"
  }
}

resource "aws_internet_gateway" "essential_igw" {
  vpc_id = aws_vpc.essential_vpc.id

  tags = {
    Name = "EssentialIGW"
  }
}

resource "aws_route_table" "essential_rt" {
  vpc_id = aws_vpc.essential_vpc.id

  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.essential_igw.id
  }

  tags = {
    Name = "EssentialRouteTable"
  }
}

resource "aws_route_table_association" "essential_rta" {
  subnet_id      = aws_subnet.essential_subnet.id
  route_table_id = aws_route_table.essential_rt.id
}


# Load Balancing

resource "aws_lb" "rqlite_lb" {
  name               = "rqlite-load-balancer"
  internal           = true
  load_balancer_type = "application"
  security_groups    = [aws_security_group.sg_rqlite_subnet.id]
  subnets            = [aws_subnet.rqlite_subnet.id]

  listener {
    instance_port     = 4001
    instance_protocol = "tcp"
    lb_port           = 4001
    lb_protocol       = "tcp"
  }
}

resource "aws_lb_target_group" "rqlite_tg" {
  name     = "rqlite-target-group"
  port     = 4001
  protocol = "TCP"
  vpc_id   = aws_vpc.essential_vpc.id

  health_check {
    protocol = "TCP"
    port     = "traffic-port"
    interval = 30
    healthy_threshold   = 3
    unhealthy_threshold = 3
  }
}

resource "aws_lb_listener" "rqlite_listener" {
  load_balancer_arn = aws_lb.rqlite_lb.arn
  port              = 4001
  protocol          = "TCP"

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.rqlite_tg.arn
  }
}

# DNS
resource "aws_route53_zone" "private_zone" {
  name = "essential.internal"

  vpc {
    vpc_id = aws_vpc.essential_vpc.id 
  }
}

resource "aws_route53_record" "private_lb_record" {
  zone_id = aws_route53_zone.private_zone.zone_id
  name    = "rqlite.essential.internal"
  type    = "A"

  alias {
    name                   = aws_lb.rqlite_lb.dns_name
    zone_id                = aws_lb.rqlite_lb.zone_id
    evaluate_target_health = true
  }
}


# Security Groups

resource "aws_security_group" "sg_essential_subnet" {
  vpc_id = aws_vpc.essential_vpc.id

  ingress {
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]  # Allow from anywhere
  }
  
  ingress {
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]  # Allow from anywhere
  }

  tags = {
    Name = "Essential_SG_Subnet"
  }
}

resource "aws_security_group" "sg_rqlite_subnet" {
  vpc_id = aws_vpc.essential_vpc.id

  ingress {
    from_port   = 4001
    to_port     = 4001
    protocol    = "tcp"
    security_groups = [aws_security_group.sg_essential_subnet.id]
  }

  # Allow all traffic within Subnet B
  ingress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    self        = true
  }

  tags = {
    Name = "Rqlite_SG_Subnet"
  }
}

# AMIs

resource "aws_ami" "rqlite_ami" {
  name                = "rqlite_server_ami"
  virtualization_type = "hvm"
  root_device_name    = "/dev/xvda"
  ebs_block_device {
    device_name = "/dev/xvda"
    snapshot_id = aws_ebs_snapshot_import.rqlite_import.id
  }
}

resource "aws_ami" "essential_ami" {
  name                = "essential_server_ami"
  virtualization_type = "hvm"
  root_device_name    = "/dev/xvda"
  ebs_block_device {
    device_name = "/dev/xvda"
    snapshot_id = aws_ebs_snapshot_import.essential_import.id
  }
}

resource "aws_ami" "etcd_ami" {
  name                = "etcd_server_ami"
  virtualization_type = "hvm"
  root_device_name    = "/dev/xvda"
  ebs_block_device {
    device_name = "/dev/xvda"
    snapshot_id = aws_ebs_snapshot_import.etcd_import.id
  }
}

resource "aws_s3_bucket" "rqlite_bucket" {}
resource "aws_s3_bucket" "essential_bucket" {}
resource "aws_s3_bucket" "etcd_bucket" {}

# AMI Upload 

resource "aws_s3_object" "rqlite_image_upload" {
  bucket = aws_s3_bucket.rqlite_bucket.id
  key    = "rqlite.vhd"
  source = var.rqlite_img_path
}

resource "aws_s3_object" "essential_image_upload" {
  bucket = aws_s3_bucket.essential_bucket.id
  key    = "essential.vhd"
  source = var.essential_img_path
}

resource "aws_s3_object" "etcd_image_upload" {
  bucket = aws_s3_bucket.etcd_bucket.id
  key    = "etcd.vhd"
  source = var.etcd_img_path
}

# AMI Import

resource "aws_ebs_snapshot_import" "rqlite_import" {
  role_name = aws_iam_role.vmimport_role.id
  disk_container {
    format = "VHD"
    user_bucket {
      s3_bucket = aws_s3_bucket.rqlite_bucket.id
      s3_key    = aws_s3_object.rqlite_image_upload.id
    }
  }
  lifecycle {
    replace_triggered_by = [
      aws_s3_object.rqlite_image_upload
    ]
  }
}

resource "aws_ebs_snapshot_import" "essential_import" {
  role_name = aws_iam_role.vmimport_role.id
  disk_container {
    format = "VHD"
    user_bucket {
      s3_bucket = aws_s3_bucket.essential_bucket.id
      s3_key    = aws_s3_object.essential_image_upload.id
    }
  }
  lifecycle {
    replace_triggered_by = [
      aws_s3_object.essential_image_upload
    ]
  }
}

resource "aws_ebs_snapshot_import" "etcd_import" {
  role_name = aws_iam_role.vmimport_role.id
  disk_container {
    format = "VHD"
    user_bucket {
      s3_bucket = aws_s3_bucket.etcd_bucket.id
      s3_key    = aws_s3_object.etcd_image_upload.id
    }
  }
  lifecycle {
    replace_triggered_by = [
      aws_s3_object.etcd_image_upload
    ]
  }
}

resource "aws_iam_role_policy_attachment" "vmpimport_attach" {
  role       = aws_iam_role.vmimport_role.id
  policy_arn = aws_iam_policy.vmimport_policy.arn
}

# IAM Roles

resource "aws_iam_role" "vmimport_role" {
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect    = "Allow"
        Principal = { Service = "vmie.amazonaws.com" }
        Action    = "sts:AssumeRole"
        Condition = {
          StringEquals = {
            "sts:Externalid" = "vmimport"
          }
        }
      }
    ]
  })
}

resource "aws_iam_policy" "vmimport_policy" {
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "s3:GetBucketLocation",
          "s3:GetObject",
          "s3:ListBucket",
          "s3:PutObject",
          "s3:GetBucketAcl"
        ]
        Resource = [
          "arn:aws:s3:::${aws_s3_bucket.rqlite_bucket.id}",
          "arn:aws:s3:::${aws_s3_bucket.rqlite_bucket.id}/*",
          "arn:aws:s3:::${aws_s3_bucket.essential_bucket.id}",
          "arn:aws:s3:::${aws_s3_bucket.essential_bucket.id}/*",
          "arn:aws:s3:::${aws_s3_bucket.etcd_bucket.id}",
          "arn:aws:s3:::${aws_s3_bucket.etcd_bucket.id}/*",
        ]
      },
      {
        Effect = "Allow"
        Action = [
          "ec2:ModifySnapshotAttribute",
          "ec2:CopySnapshot",
          "ec2:RegisterImage",
          "ec2:Describe*"
        ],
        Resource = "*"
      }
    ]
  })
}