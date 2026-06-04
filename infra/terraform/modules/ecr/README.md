# modules/ecr

Private ECR repository for the `argo` container image.

Provisions:
- An `aws_ecr_repository` named `<prefix>-argo` with `scan_on_push` enabled.
- A lifecycle policy that keeps the last 10 tagged images and expires untagged images after 7
  days.
- Image-tag mutability = `MUTABLE` (operator pushes `:latest` for trunk-deploy by default;
  pinning a SHA tag in `image_tag` switches the service to immutable rollouts).
