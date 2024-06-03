FROM node:20.12.2-slim AS base
ENV PNPM_HOME="/pnpm"
ENV PATH="$PNPM_HOME:$PATH"
RUN corepack enable
COPY . /app
WORKDIR /app

RUN --mount=type=cache,id=pnpm,target=/pnpm/store pnpm install --frozen-lockfile

FROM base AS simple-webapp
WORKDIR /app/apps/simple-webapp
EXPOSE 5173
CMD [ "pnpm", "run", "dev", "--host" ]
