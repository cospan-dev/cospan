# Stage 1: Build
FROM node:22-slim AS builder

WORKDIR /app

RUN corepack enable

# Copy workspace root files
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./

# Copy the web app and shared packages
COPY apps/web/ apps/web/
COPY packages/ packages/

RUN pnpm install --frozen-lockfile
RUN pnpm --filter @cospan/web build

# Stage 2: Runtime
FROM node:22-slim

WORKDIR /app

COPY --from=builder /app/apps/web/build ./build
COPY --from=builder /app/apps/web/package.json .

USER 1001

EXPOSE 3002

CMD ["node", "build"]
