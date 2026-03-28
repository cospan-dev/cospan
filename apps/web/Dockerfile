# Stage 1: Build
FROM node:22-slim AS builder

WORKDIR /app

RUN corepack enable

# Copy workspace root files
COPY package.json pnpm-workspace.yaml ./

# Copy the web app and shared packages
COPY apps/web/ apps/web/
COPY packages/ packages/

RUN cd apps/web && pnpm install --no-frozen-lockfile
RUN cd apps/web && pnpm build

# Stage 2: Runtime
FROM node:22-slim

WORKDIR /app

COPY --from=builder /app/apps/web/build ./build
COPY --from=builder /app/apps/web/package.json .
COPY --from=builder /app/apps/web/node_modules ./node_modules

USER 1001

EXPOSE 3000

CMD ["node", "build"]
