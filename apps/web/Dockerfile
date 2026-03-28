# Stage 1: Build
FROM node:22-slim AS builder

WORKDIR /app

RUN corepack enable

COPY package.json pnpm-workspace.yaml ./
COPY apps/web/ apps/web/
COPY packages/ packages/

RUN cd apps/web && pnpm install --no-frozen-lockfile
RUN cd apps/web && pnpm build

# Stage 2: Runtime - install production deps fresh
FROM node:22-slim

WORKDIR /app

RUN corepack enable

# Copy built output
COPY --from=builder /app/apps/web/build ./build
COPY --from=builder /app/apps/web/package.json .

# Install production dependencies only (includes @sveltejs/kit needed at runtime)
RUN pnpm install --prod --no-frozen-lockfile

USER 1001

EXPOSE 3000

CMD ["node", "build"]
